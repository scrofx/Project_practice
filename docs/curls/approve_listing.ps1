$approveBody = @{
    listing_id = "8e89e02c-a09f-46c4-a738-43ce91052d7c"
    action = "approve"
} | ConvertTo-Json -Compress

$approved = Invoke-RestMethod `
    -Method Post `
    -Uri "$baseUrl/moderation/listings" `
    -Headers $headers `
    -ContentType "application/json" `
    -Body $approveBody

Write-Host "`nApproved listing:" -ForegroundColor Green
$approved | Format-List id, title, status