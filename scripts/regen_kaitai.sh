#!/usr/bin/env bash
# Regenerate the NCS Kaitai parser from the authoritative spec.
# Requires: kaitai-struct-compiler 0.11 (https://github.com/kaitai-io/kaitai_struct_compiler)
# and a JDK 21. The generated file is committed so normal `cargo build` needs NEITHER.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
ksc="${KSC:-kaitai-struct-compiler}"
"$ksc" --target rust --outdir "$here/ncs_tool/src/kaitai" "$here/decompiled_validators/ncs.ksy"
echo "regenerated ncs_tool/src/kaitai/ncs_session.rs from decompiled_validators/ncs.ksy"
