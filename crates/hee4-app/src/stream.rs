//! `events.subscribe {since_seq, epoch}` (I5): one JSON line per ledger event, `seq` ascending,
//! replayed from `since_seq` and then followed live, until the client closes. The cursor check
//! (R13, `Store::cursor_check`) and the ack `{since_seq, epoch, high_water, stream}` are the
//! handler's (`actions/task.rs`); this module starts after the ack is written.
//!
//! Three threads per subscriber: the connection thread writes frames; a reader thread polls
//! the ledger through its own read-only SQLite connection and offers frames to a bounded queue;
//! a close watcher reads the socket until EOF. The reader never waits on the subscriber: a
//! full queue drops the subscriber with a `slow_consumer` close frame. The `Store` mutex is
//! taken only to decode a batch's histories (`Store::history`, the one event decoder).

use std::collections::HashMap;
use std::io::{Read as _, Write as _};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel};
use std::time::Duration;

use hee4_contracts::{Event, StreamBudget, TaskId, TaskState};
use rusqlite::{Connection, OpenFlags, params};
use serde_json::{Value, json};

use crate::actions::Engine;
use crate::wire::{self, Code};

/// Why the reader stopped offering frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// The queue was full: the subscriber is a slow consumer.
    SlowConsumer,
    /// The writer is gone (client closed).
    Gone,
}

/// Offer `frames` without waiting. A full queue is [`Stop::SlowConsumer`].
///
/// # Errors
/// [`Stop`].
pub fn offer(tx: &SyncSender<Value>, frames: Vec<Value>) -> Result<(), Stop> {
    for frame in frames {
        match tx.try_send(frame) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => return Err(Stop::SlowConsumer),
            Err(TrySendError::Disconnected(_)) => return Err(Stop::Gone),
        }
    }
    Ok(())
}

/// One `events` row: global `seq`, its task, its timestamp, and its 1-based position in the
/// task's own history.
struct Row {
    seq: i64,
    task: String,
    ts: i64,
    nth: usize,
}

/// Up to `batch` (`stream.batch_rows`) rows after `after`, in `seq` order.
fn rows_after(conn: &Connection, after: i64, batch: i64) -> rusqlite::Result<Vec<Row>> {
    let mut stmt = conn.prepare_cached(
        "SELECT e.seq, e.task_id, e.ts,
                (SELECT count(*) FROM events x WHERE x.task_id = e.task_id AND x.seq <= e.seq)
         FROM events e WHERE e.seq > ?1 ORDER BY e.seq LIMIT ?2",
    )?;
    stmt.query_map(params![after, batch], |r| {
        Ok(Row {
            seq: r.get(0)?,
            task: r.get(1)?,
            ts: r.get(2)?,
            nth: usize::try_from(r.get::<_, i64>(3)?).unwrap_or(0),
        })
    })?
    .collect()
}

/// The frames for `rows`: `{kind:"event", seq, task_id, event, phase_after, ts}`.
fn frames(engine: &Engine, rows: &[Row]) -> Result<Vec<Value>, String> {
    let mut histories: HashMap<&str, Vec<Event>> = HashMap::new();
    {
        let store = engine.store();
        for row in rows {
            if !histories.contains_key(row.task.as_str()) {
                let task: TaskId = row.task.parse().map_err(|e| format!("task_id: {e}"))?;
                let h = store.history(&task).map_err(|e| e.to_string())?;
                histories.insert(row.task.as_str(), h);
            }
        }
    }
    rows.iter()
        .map(|row| {
            let history = histories
                .get(row.task.as_str())
                .map_or(&[][..], Vec::as_slice);
            let prefix = history
                .get(..row.nth)
                .filter(|p| !p.is_empty())
                .ok_or_else(|| format!("seq {} has no history entry {}", row.seq, row.nth))?;
            let event = prefix[prefix.len() - 1];
            let state = TaskState::replay(prefix.iter().copied()).map_err(|e| e.to_string())?;
            Ok(json!({
                "kind": "event",
                "seq": row.seq,
                "task_id": row.task,
                "event": event,
                "phase_after": state.phase().as_str(),
                "ts": row.ts,
            }))
        })
        .collect()
}

/// The reader thread: poll the ledger after `cursor`, offer frames, until stopped.
fn read_ledger(
    engine: &Engine,
    mut cursor: i64,
    tx: &SyncSender<Value>,
    closed: &AtomicBool,
) -> Result<Stop, String> {
    let conn = Connection::open_with_flags(
        engine.ledger(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;
    let budget = engine.budgets().stream;
    let batch = i64::try_from(budget.batch_rows).unwrap_or(i64::MAX);
    while !closed.load(Ordering::Acquire) {
        let rows = rows_after(&conn, cursor, batch).map_err(|e| e.to_string())?;
        let Some(last) = rows.last().map(|r| r.seq) else {
            std::thread::sleep(budget.poll());
            continue;
        };
        if let Err(stop) = offer(tx, frames(engine, &rows)?) {
            return Ok(stop);
        }
        cursor = last;
    }
    Ok(Stop::Gone)
}

/// Send the `slow_consumer` close frame, best effort, under `deadline`
/// (`stream.close_deadline_ms`: the peer's buffer is probably full, so the write must not wait
/// on it): when the write misses it the peer sees EOF and the server logs it.
fn write_close(writer: &mut UnixStream, deadline: Duration) -> std::io::Result<()> {
    let close = wire::close(Code::SlowConsumer, "queue full; resubscribe with since_seq");
    let sent = writer
        .set_write_timeout(Some(deadline))
        .and_then(|()| writer.write_all(format!("{close}\n").as_bytes()));
    if let Err(e) = &sent {
        eprintln!("slow_consumer close frame not delivered ({e})");
    }
    sent
}

/// Write frames from `rx` until the client closes or the reader stops; a slow consumer gets
/// a `slow_consumer` close frame first. `budget` is the engine's stream budget.
fn write_frames(
    writer: &mut UnixStream,
    rx: &Receiver<Value>,
    slow: &AtomicBool,
    closed: &AtomicBool,
    budget: StreamBudget,
) -> std::io::Result<()> {
    loop {
        if slow.load(Ordering::Acquire) {
            return write_close(writer, budget.close_deadline());
        }
        match rx.recv_timeout(budget.poll()) {
            Ok(frame) => writer.write_all(format!("{frame}\n").as_bytes())?,
            Err(RecvTimeoutError::Timeout) if closed.load(Ordering::Acquire) => return Ok(()),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) if slow.load(Ordering::Acquire) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

/// Run a subscription on `writer` (the ack is already written). Returns when the stream ends.
///
/// # Errors
/// IO on the socket.
pub fn run(mut writer: UnixStream, engine: Arc<Engine>, since_seq: i64) -> std::io::Result<()> {
    writer.set_read_timeout(None)?;
    let closed = Arc::new(AtomicBool::new(false));
    let slow = Arc::new(AtomicBool::new(false));
    let mut watch = writer.try_clone()?;
    let watch_closed = Arc::clone(&closed);
    std::thread::spawn(move || {
        let mut sink = [0_u8; 512];
        while matches!(watch.read(&mut sink), Ok(n) if n > 0) {}
        watch_closed.store(true, Ordering::Release);
    });
    let budget = engine.budgets().stream;
    // `stream.queue_frames`: frames a subscriber may fall behind before it is dropped (API Map
    // A-10 queue bound).
    let (tx, rx) = sync_channel(usize::try_from(budget.queue_frames).unwrap_or(usize::MAX));
    let reader_closed = Arc::clone(&closed);
    let reader_slow = Arc::clone(&slow);
    std::thread::spawn(
        move || match read_ledger(&engine, since_seq, &tx, &reader_closed) {
            Ok(Stop::SlowConsumer) => {
                eprintln!("events.subscribe dropped: slow_consumer");
                reader_slow.store(true, Ordering::Release);
            }
            Ok(Stop::Gone) => {}
            Err(e) => eprintln!("events.subscribe reader stopped: {e}"),
        },
    );
    let result = write_frames(&mut writer, &rx, &slow, &closed, budget);
    if result.is_err() && slow.load(Ordering::Acquire) {
        // The writer itself was stuck on a full peer buffer, so the close frame was never tried.
        eprintln!("slow_consumer close frame not delivered (writer blocked)");
    }
    closed.store(true, Ordering::Release);
    let _ = writer.shutdown(Shutdown::Both);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_close_frame_arrives_when_the_peer_reads_and_is_logged_when_it_cannot()
    -> std::io::Result<()> {
        use std::io::{BufRead as _, BufReader};
        let (mut server, client) = UnixStream::pair()?;
        assert!(write_close(&mut server, StreamBudget::DEFAULT.close_deadline()).is_ok());
        let mut line = String::new();
        BufReader::new(&client).read_line(&mut line)?;
        assert!(line.contains("slow_consumer"), "{line}");

        // A peer that never reads: fill the buffer until a write would block, then the close
        // frame misses its deadline and the function reports it (and logs the line).
        let (mut server, _client) = UnixStream::pair()?;
        server.set_nonblocking(true)?;
        while server.write(&[b'x'; 4096]).is_ok() {}
        server.set_nonblocking(false)?;
        let t0 = std::time::Instant::now();
        assert!(write_close(&mut server, StreamBudget::DEFAULT.close_deadline()).is_err());
        assert!(t0.elapsed() < Duration::from_secs(2), "{:?}", t0.elapsed());
        Ok(())
    }

    #[test]
    fn a_full_queue_is_a_slow_consumer_not_a_wait() {
        let (tx, rx) = sync_channel(1);
        assert_eq!(
            offer(&tx, vec![json!(1), json!(2)]),
            Err(Stop::SlowConsumer)
        );
        assert_eq!(rx.try_recv().ok(), Some(json!(1)));
        drop(rx);
        assert_eq!(offer(&tx, vec![json!(3)]), Err(Stop::Gone));
    }
}
