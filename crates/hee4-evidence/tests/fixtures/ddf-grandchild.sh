#!/bin/sh
# Stub: forks a sleeping grandchild, writes its pid to the path given on stdin, then never exits
# within a test budget. Without a process-group kill the grandchild outlives the deadline.
read -r pidfile
sleep 30 &
echo $! > "$pidfile.tmp" && mv "$pidfile.tmp" "$pidfile"
wait
