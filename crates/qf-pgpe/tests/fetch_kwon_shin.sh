#!/usr/bin/env bash
# Download (with retries: the Zenodo gateway returns 502/504 routinely), verify and extract the files of the Kwon & Shin
# reference run that the reproduction test needs.   usage: fetch_kwon_shin.sh OUTDIR
set -euo pipefail
OUT="${1:?usage: fetch_kwon_shin.sh OUTDIR}"
URL="https://zenodo.org/api/records/20068724/files/Vortex-shedding-PRR-data-1.0.0.zip/content"
SHA256="69a58532fc01a013a2a7ba17428787f68e2c62b4a5181cee4e822092d7aa5b9e"
mkdir -p "$OUT"
ZIP="$OUT/reference.zip"
for attempt in 1 2 3 4 5 6; do
  if curl -fsSL --retry 3 --retry-delay 5 -o "$ZIP" "$URL" && echo "$SHA256  $ZIP" | sha256sum -c --quiet -; then
    break
  fi
  echo "download attempt $attempt failed or checksum mismatch; retrying" >&2
  sleep $((attempt * 10))
  [ "$attempt" = 6 ] && { echo "giving up" >&2; exit 1; }
done
unzip -o -j -q "$ZIP" '*/outputs/*/Psi/psi_time_0.0.npy' '*/outputs/*/Force/force_dt=0.02.txt' -d "$OUT"
ls -l "$OUT"
