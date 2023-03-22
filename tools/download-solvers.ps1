# Copyright 2022-2023 VMware, Inc.
# SPDX-License-Identifier: BSD-2-Clause

# This script downloads binary releases of Z3, CVC4, and cvc5 to solvers/ in
# the source directory, according to the versions specified in
# tools/solver-versions.ps1. It handles 64-bit Windows.

$ErrorActionPreference = "Stop"

. $PSScriptRoot\solver-versions.ps1
if ($Env:OS -ne "Windows_NT" -Or ![Environment]::Is64BitOperatingSystem) {
    echo "This script is only for 64-bit Windows!"
    exit
}

$Z3File = "z3-$Z3Version-x64-win"
$CVC4File = "cvc4-$CVC4Version-win64-opt.exe"
$CVC5File = "cvc5-Win64.exe"

$Z3Url="https://github.com/Z3Prover/z3/releases/download/z3-${Z3Version}/${Z3File}.zip"
$CVC4Url="https://github.com/CVC4/CVC4/releases/download/${CVC4Version}/${CVC4File}"
$CVC5Url="https://github.com/cvc5/cvc5/releases/download/cvc5-${CVC5Version}/${CVC5File}"

New-Item -ItemType Directory -Path solvers -ErrorAction Ignore
$ProgressPreference = 'SilentlyContinue'


if ((Get-Item -Path "solvers/z3.exe" -ErrorAction SilentlyContinue) -And
 (.\solvers\z3.exe --version | Select-String -SimpleMatch -Quiet $Z3Version)) {
    echo "found Z3 $Z3Version"
} else {
    echo "downloading Z3 from ${Z3Url}"
    Invoke-WebRequest $Z3Url -OutFile solvers/z3.zip
    Expand-Archive -Path solvers/z3.zip -DestinationPath solvers
    Remove-Item -Path solvers/z3.zip
    Move-Item -Path solvers/$Z3File/bin/z3.exe -Destination solvers/z3.exe
    Remove-Item -Recurse -Path solvers/$Z3File
}

if ((Get-Item -Path "solvers/cvc5.exe" -ErrorAction SilentlyContinue) -And
 (.\solvers\cvc5.exe --version | Select-String -SimpleMatch -Quiet $CVC5Version)) {
    echo "found CVC5 $CVC5Version"
} else {
    echo "downloading CVC5 from ${CVC5Url}"
    Invoke-WebRequest $CVC5Url -OutFile solvers/cvc5.exe
}

if ((Get-Item -Path "solvers/cvc4.exe" -ErrorAction SilentlyContinue) -And
 (.\solvers\cvc4.exe --version | Select-String -SimpleMatch -Quiet $CVC4Version)) {
    echo "found CVC4 $CVC4Version"
} else {
    echo "downloading CVC4 from ${CVC4Url}"
    Invoke-WebRequest $CVC4Url -OutFile solvers/cvc4.exe
}
