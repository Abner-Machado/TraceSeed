#!/bin/sh
# capture -> failure.trace -> edit a field -> replay
set -eu
cd "$(dirname "$0")"

step() { printf '\n== %s\n\n' "$*"; }

step "1. capture: run the example until it fails, then freeze that run"
TRACE_ENV=CURRENCY CURRENCY=BRL ./trace-seed capture ./example.sh payment:100

step "2. the capsule"
cat failure.trace

step "3. replay: same file, same failure, as many times as you like"
./trace-seed replay failure.trace || true

step "4. edit one field: env.CURRENCY BRL -> USD"
sed -i.bak 's/^env\.CURRENCY=BRL$/env.CURRENCY=USD/' failure.trace
./trace-seed replay failure.trace || true

step "5. put the currency back, nudge the seed instead"
mv failure.trace.bak failure.trace
sed -i "s/^seed=.*/seed=$(( $(sed -n 's/^seed=//p' failure.trace) + 1 ))/" failure.trace
./trace-seed replay failure.trace || true

printf '\n== the bug needs BRL and that one seed. Two edits, two answers.\n'
