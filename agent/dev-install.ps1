$ErrorActionPreference = 'Stop'

$jarvisDirectory = "$HOME\.jarvis"
$pluginsDirectory = Join-Path -Path $jarvisDirectory -ChildPath "agent-plugins"
$linuxArch = "x86_64-unknown-linux-gnu"

Function Install-JarvisBinary($BaseDirectory, $TargetArchitecture, $Source, $Name) {
    If (Test-Path -Path $Source) {
        $binaryDirectory = Join-Path -Path $BaseDirectory -ChildPath "$Name\0.0.0-dev\$TargetArchitecture"

        New-Item -ItemType Directory -Path $binaryDirectory -Force

        tar -xvf $Source -C $binaryDirectory
    }
}

Install-JarvisBinary -BaseDirectory $jarvisDirectory -TargetArchitecture $linuxArch -Source "agent-worker-x86_64-unknown-linux-gnu.tar" -Name "agent-worker"

Install-JarvisBinary -BaseDirectory $pluginsDirectory -TargetArchitecture $linuxArch -Source "hello-world-plugin-x86_64-unknown-linux-gnu.tar" -Name "hello-world-plugin"
