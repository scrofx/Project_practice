$removeBody = @{
    listing_id = "12c0c272-01c1-4dd6-aa3a-3725c18f6a8d"
    action = "remove"
    reason = "Test"
} | ConvertTo-Json -Compress

$removed = Invoke-RestMethod `
    -Method Post `
    -Uri "$baseUrl/moderation/listings" `
    -Headers $headers `
    -ContentType "application/json" `
    -Body $removeBody

Write-Host "`nRemoved listing:" -ForegroundColor Yellow
$removed | Format-List id, title, status