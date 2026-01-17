# Kill process on port 8021 if it exists
$process = Get-NetTCPConnection -LocalPort 8021 -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess | Select-Object -Unique
if ($process) {
    Write-Host "Killing process on port 8021: $process"
    Stop-Process -Id $process -Force -ErrorAction SilentlyContinue
}

# Start SurrealDB
& "d:\work\plant-code\surrealdb\target\release\surreal.exe" start --user root --pass root --bind 0.0.0.0:8021 surrealkv://ams-8021.kv
