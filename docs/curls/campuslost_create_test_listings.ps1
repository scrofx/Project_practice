
$ErrorActionPreference = "Stop"
$BaseUrl = "http://localhost:8000"
$Password = "CampusLost123!"

# Fetch catalog data
$categories = Invoke-RestMethod -Uri "$BaseUrl/categories" -Method Get
$buildings  = Invoke-RestMethod -Uri "$BaseUrl/buildings" -Method Get

function Get-CategoryId([string]$slug) {
    $item = $categories | Where-Object { $_.slug -eq $slug } | Select-Object -First 1
    if (-not $item) { throw "Category '$slug' not found. Check that category seed SQL was executed." }
    return $item.id
}

function Get-BuildingId([string]$code) {
    $item = $buildings | Where-Object { $_.code -eq $code } | Select-Object -First 1
    if (-not $item) { throw "Building '$code' not found. Check that building seed SQL was executed." }
    return $item.id
}

# Log in once per student and cache the token.
$tokens = @{}
function Get-StudentToken([string]$email) {
    if (-not $tokens.ContainsKey($email)) {
        $loginBody = @{
            email    = $email
            password = $Password
        } | ConvertTo-Json

        $login = Invoke-RestMethod `
            -Uri "$BaseUrl/auth/login" `
            -Method Post `
            -ContentType "application/json" `
            -Body $loginBody

        $tokens[$email] = $login.access_token
        Write-Host "Logged in: $email" -ForegroundColor DarkGray
    }
    return $tokens[$email]
}

$today = Get-Date

# 12 listings, spread across 8 students, categories, buildings, types and dates.
$testListings = @(
    @{
        email = "alex.morgan@campuslost.test"; type = "lost"
        category = "keys"; building = "DORM-01"
        title = "Lost bunch of keys with blue tag"
        description = "A bunch of three keys with a small blue plastic tag. Might have been dropped near the entrance."
        location = "Near the main entrance"
        daysAgo = 1
    },
    @{
        email = "emma.wilson@campuslost.test"; type = "found"
        category = "keys"; building = "DORM-01"
        title = "Found silver keys"
        description = "Found two silver keys on a metal ring near the stairs."
        location = "Second floor, staircase"
        daysAgo = 1
    },
    @{
        email = "daniel.brown@campuslost.test"; type = "lost"
        category = "electronics"; building = "DORM-02"
        title = "Lost black wireless headphones"
        description = "Black wireless headphones in a charging case. The case has a small scratch."
        location = "Study room"
        daysAgo = 3
    },
    @{
        email = "sophia.taylor@campuslost.test"; type = "found"
        category = "electronics"; building = "DORM-05"
        title = "Found USB-C charger"
        description = "White USB-C laptop charger left plugged into a socket."
        location = "Kitchen, near the window"
        daysAgo = 2
    },
    @{
        email = "oliver.davis@campuslost.test"; type = "lost"
        category = "access-cards"; building = "DORM-06"
        title = "Lost student access card"
        description = "Student access card with a dark blue holder. Please contact me if found."
        location = "Near the reception"
        daysAgo = 5
    },
    @{
        email = "mia.anderson@campuslost.test"; type = "found"
        category = "bags-backpacks"; building = "DORM-07"
        title = "Found black backpack"
        description = "Black backpack left unattended. It contains notebooks, no visible name."
        location = "Common room"
        daysAgo = 4
    },
    @{
        email = "ethan.thomas@campuslost.test"; type = "lost"
        category = "clothing-accessories"; building = "DORM-08"
        title = "Lost grey hoodie"
        description = "Grey hoodie, size M, with a small white logo on the chest."
        location = "Laundry room"
        daysAgo = 7
    },
    @{
        email = "isabella.martin@campuslost.test"; type = "found"
        category = "study-materials"; building = "DORM-09"
        title = "Found calculus notebook"
        description = "Blue notebook with handwritten calculus notes. No name on the cover."
        location = "Desk near the entrance"
        daysAgo = 6
    },
    @{
        email = "alex.morgan@campuslost.test"; type = "lost"
        category = "personal-items"; building = "DORM-10"
        title = "Lost black wallet"
        description = "Small black wallet. It may contain student cards; please do not publish personal details."
        location = "Near the elevator"
        daysAgo = 10
    },
    @{
        email = "emma.wilson@campuslost.test"; type = "found"
        category = "sports-equipment"; building = "NVK-01"
        title = "Found a tennis racket"
        description = "Tennis racket in a dark cover, left beside the building entrance."
        location = "Ground floor"
        daysAgo = 12
    },
    @{
        email = "daniel.brown@campuslost.test"; type = "lost"
        category = "bags-backpacks"; building = "NVK-02"
        title = "Lost green university backpack"
        description = "Dark green backpack with a laptop compartment and a keychain."
        location = "Near the bike racks"
        daysAgo = 14
    },
    @{
        email = "sophia.taylor@campuslost.test"; type = "found"
        category = "access-cards"; building = "NVK-04"
        title = "Found campus access card"
        description = "A campus access card found near the entrance. The card is being kept safe."
        location = "Entrance hall"
        daysAgo = 20
    }
)

$created = @()

foreach ($item in $testListings) {
    $token = Get-StudentToken $item.email

    $body = @{
        type           = $item.type
        category_id    = Get-CategoryId $item.category
        building_id    = Get-BuildingId $item.building
        title          = $item.title
        description    = $item.description
        location_detail = $item.location
        event_date     = $today.AddDays(-$item.daysAgo).ToString("yyyy-MM-dd")
    } | ConvertTo-Json

    $response = Invoke-RestMethod `
        -Uri "$BaseUrl/listings" `
        -Method Post `
        -ContentType "application/json" `
        -Headers @{ Authorization = "Bearer $token" } `
        -Body $body

    $created += $response
    Write-Host ("Created: {0} | {1} | {2}" -f $response.title, $response.type, $response.id) -ForegroundColor Green
}

Write-Host ""
Write-Host "Created $($created.Count) listings." -ForegroundColor Cyan
Write-Host "All of them should currently have status: pending_review"
$created | Select-Object id, title, type, category_id, building_id, event_date, status | Format-Table -AutoSize
