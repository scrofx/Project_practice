
$baseUrl = "http://127.0.0.1:8000"

$loginBody = @{
    email = "james.wilson@campuslost.test"
    password = "CampusLost123!"
}

$login = Invoke-RestMethod `
    -Method Post `
    -Uri "$baseUrl/auth/login" `
    -ContentType "application/json" `
    -Body ($loginBody | ConvertTo-Json -Compress)

$token = $login.access_token

Write-Host "Login successful!" -ForegroundColor Green
Write-Host "Token: $token"

$headers = @{
    Authorization = "Bearer $token"
}

