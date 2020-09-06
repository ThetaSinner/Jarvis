$jarvisDirectory = "$HOME\.jarvis\"
New-Item -ItemType Directory -Path $jarvisDirectory -Force

Copy-Item -Path ".\target\debug\agent-worker.exe" -Destination $jarvisDirectory -Force -ErrorAction 'silentlycontinue'
