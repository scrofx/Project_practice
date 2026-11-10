$baseUrl = "http://127.0.0.1:8000"

$publicListings = Invoke-RestMethod `
    -Method Get `
    -Uri "$baseUrl/listings"

Write-Host "`nPublic listings:" -ForegroundColor Cyan
$publicListings | Format-Table id, title, status -AutoSize