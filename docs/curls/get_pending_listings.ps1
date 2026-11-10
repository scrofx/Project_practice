$pending = Invoke-RestMethod `
    -Method Get `
    -Uri "$baseUrl/moderation/listings" `
    -Headers $headers

Write-Host "`nPending listings:" -ForegroundColor Cyan

$pending | Format-Table id, title, created_at -AutoSize