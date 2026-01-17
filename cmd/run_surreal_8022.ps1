# Kill process on port 8022 if it exists
$process = Get-NetTCPConnection -LocalPort 8020 -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess | Select-Object -Unique
if ($process) {
    Write-Host "Killing process on port 8020: $process"
    Stop-Process -Id $process -Force -ErrorAction SilentlyContinue
}

# Start SurrealDB with RocksDB
& "surreal.exe" start --user root --pass root --bind 0.0.0.0:8020 rocksdb://ams-8020.db
