#!/bin/sh
# Stub: forks a sleeping grandchild that inherits stdout/stderr and writes its pid to the path given on
# stdin. Mode `wait` (the default) then never exits within a test budget; mode `exit` exits 0 at once,
# leaving the grandchild holding the pipes; mode `escape` does the same with a grandchild that left the
# process group (`setsid`). Without a process-group kill the grandchild outlives the call.
read -r pidfile mode
if [ "$mode" = escape ]; then
    setsid sleep 30 &
else
    sleep 30 &
fi
echo $! > "$pidfile.tmp" && mv "$pidfile.tmp" "$pidfile"
if [ "$mode" = exit ] || [ "$mode" = escape ]; then
    exit 0
fi
wait
