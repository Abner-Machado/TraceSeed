#!/bin/sh
# A payment check with a rare bug. Deterministic: the same SEED always
# produces the same result, so a failure can be pinned down and replayed.
set -u

amount=${INPUT##*:}
currency=${CURRENCY:-USD}

# Which fee bracket this payment lands in.
bracket=$(( (SEED * 7919 + 104729) % 100 ))
fee=$(( amount * bracket / 1000 ))

echo "payment=$amount currency=$currency bracket=$bracket fee=$fee"

# The BRL rate table has a gap: brackets under 3 were never filled in.
if [ "$currency" = "BRL" ] && [ "$bracket" -lt 3 ]; then
    echo "ERROR: no $currency rate for bracket $bracket" >&2
    exit 1
fi
