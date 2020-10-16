$ErrorActionPreference = 'Stop'

$jarvisDirectory = "$HOME\.jarvis"
$binDirectory = Join-Path -Path $jarvisDirectory -ChildPath "bin"

if (-Not (Test-Path -Path $binDirectory)) {
    New-Item -ItemType Directory -Path $binDirectory
}

Copy-Item detect_arch.sh $binDirectory
