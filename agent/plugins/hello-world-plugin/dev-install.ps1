$pluginName = "hello-world-plugin"

$pluginDirectory = "$HOME\.jarvis\agent-plugins\$pluginName\0.1.0-dev\"
New-Item -ItemType Directory -Path $pluginDirectory -Force

Copy-Item -Path ".\target\debug\$pluginName.exe" -Destination $pluginDirectory -Force -ErrorAction 'silentlycontinue'
