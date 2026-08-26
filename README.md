# TraceSeed

Turn a failing run into a small text file you can edit and run again.

```
TEST -> FAIL -> CAPTURE -> .trace -> EDIT -> REPLAY
```

## The problem

A test fails on CI. You run it again locally and it passes. The failure happened under
some combination of a random seed, an input and a couple of environment variables, and
none of that survived the run — the log tells you *that* it broke, not *what it took* to
break it.

The usual answers are to add more logging, or to freeze the whole world in a container.
TraceSeed does something smaller: it writes down the conditions, and nothing else.

## The idea

A failure is not an event to read about. It is a set of inputs — so store it as inputs.

```
command=./example.sh
seed=27209
input=payment:100
env.CURRENCY=BRL
exit=1
```

That is the entire artifact. Five lines, plain text. Commit it next to the test, diff it
in a review, edit it in any editor, hand it to someone else and they get the same failure
you got.

And because you can edit it, the file stops being a record and becomes an instrument:
change one field, replay, and you have tested a hypothesis about the bug.

## Every field answers one question

| Field     | Question                        |
| --------- | ------------------------------- |
| `command` | What was executed?              |
| `seed`    | Under which random draw?        |
| `input`   | With which input?               |
| `env.*`   | With which environment?         |
| `exit`    | What counts as reproducing it?  |

Nothing else is captured. A field that does not change the outcome does not belong in
the file.

## Try it

Requires a Rust toolchain and a POSIX shell. There are no dependencies.

```bash
git clone https://github.com/Abner-Machado/TraceSeed
cd TraceSeed
./demo.sh
```

`example.sh` is a payment check with a rare bug: a gap in the BRL rate table that only
shows up for some fee brackets. It fails on roughly 3% of seeds — the kind of test that
passes when you rerun it.

```bash
$ TRACE_ENV=CURRENCY CURRENCY=BRL ./trace-seed capture ./example.sh payment:100
payment=100 currency=BRL bracket=0 fee=0
ERROR: no BRL rate for bracket 0

captured after 58 runs -> failure.trace
```

The capsule:

```bash
$ cat failure.trace
command=./example.sh
seed=27209
input=payment:100
env.CURRENCY=BRL
exit=1
```

Replay it as often as you like:

```bash
$ ./trace-seed replay failure.trace
payment=100 currency=BRL bracket=0 fee=0
ERROR: no BRL rate for bracket 0
REPRODUCED (exit 1, as recorded)
```

Now edit the file. Change `env.CURRENCY` to `USD`:

```bash
$ ./trace-seed replay failure.trace
payment=100 currency=USD bracket=0 fee=0
NOT REPRODUCED (exit 0, the trace records exit 1)
```

Put the currency back and nudge the seed by one instead:

```bash
$ ./trace-seed replay failure.trace
payment=100 currency=BRL bracket=19 fee=1
NOT REPRODUCED (exit 0, the trace records exit 1)
```

Two edits, two answers: the bug needs BRL, and it needs that bracket. That is a
bisection, done with a text editor.

## How a program plugs in

The contract is three environment variables, so anything that runs in a shell can use it:

- `SEED` — the run's seed. Your program derives its randomness from this instead of the clock.
- `INPUT` — the run's input.
- every `env.*` field from the trace, exported by name.

`capture` runs the command up to 200 times with a new seed each time and stops at the
first non-zero exit. `TRACE_ENV` lists the environment variables worth recording —
declaring them keeps the trace to what actually matters.

`replay` exits `0` when the recorded exit code comes back and `1` when it does not, so a
trace also works as a regression check: run it in CI and it tells you the day the bug
stops reproducing.

## Limitations

This is a proof of concept, and the boundaries are the point:

- **Determinism is the program's job.** TraceSeed hands you a seed; if your code reads the
  clock, hits the network or races threads, the seed will not save you.
- **The oracle is the exit code.** A wrong answer that still exits `0` is invisible here.
- **No filesystem or database state** is captured. A failure that depends on what was in
  a table will not reproduce from these five lines.
- **The command runs through `sh -c`**, so a POSIX shell has to exist.

## License

MIT
