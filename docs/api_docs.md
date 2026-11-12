### `POST /auth/register`

Registers a new user account and returns an authentication response.

**Authentication:** Not required.

**Request body:** `RegisterRequest` (JSON)

| Field | Type | Required | Description |
|---|---|---|---|
| `email` | string | Yes | User email address |
| `full_name` | string | Yes | User's full name |
| `password` | string | Yes | Password; minimum 8 characters |
| `phone` | string? | No | Phone number |
| `telegram_handle` | string? | No | Telegram username; leading `@` is removed |
| `vk_handle` | string? | No | VK username |
| `preferred_contact` | `ContactMethod`? | No | Preferred contact method; defaults to `email` |

If `phone`, `telegram`, or `vk` is selected as the preferred contact method, the corresponding contact value must be provided.

**Success response:** `201 Created`

Returns `AuthResponse` with the newly created user's authentication data.

**Errors:**
- `400 Bad Request` — `invalid_email`, `invalid_full_name`, `weak_password`, or `invalid_preferred_contact`.
- `409 Conflict` — `email_already_registered`.
- `500 Internal Server Error` — `database_error`.

**Notes:**
- Passwords are hashed before being stored.
- Public registration always assigns the `student` role.
- New accounts are active and have notifications enabled.

### `POST /auth/login`

Authenticates a user and returns an authentication response.

**Authentication:** Not required.

**Request body:** `LoginRequest` (JSON)

| Field | Type | Required | Description |
|---|---|---|---|
| `email` | string | Yes | User email address |
| `password` | string | Yes | Account password |

**Success response:** `200 OK`

Returns `AuthResponse` containing the authentication data.

**Errors:**
- `401 Unauthorized` — `invalid_credentials` if the email or password is incorrect, or the account is inactive.

**Notes:**
- The email is trimmed before lookup.
- The password is verified against its stored hash.
- The same error is returned for an unknown email, an incorrect password, or an inactive account.


### Authentication and role guards

The API uses JWT-based authentication. Protected routes use `AuthUser`, `StudentGuard`, `ModeratorGuard`, or `AdminGuard` as request guards.

**Authentication (`AuthUser`):**
- Requires an `Authorization` header in the format `Bearer <token>`.
- Validates the JWT using `HS256` and the `JWT_SECRET` environment variable.
- `JWT_SECRET` must contain at least 32 characters.
- Checks that the user exists, is active, and has the same role as specified in the token.
- Loads the authenticated user from the database.

**Role guards:**

| Guard | Allowed roles |
|---|---|
| `StudentGuard` | `student` |
| `ModeratorGuard` | `moderator`, `admin` |
| `AdminGuard` | `admin` |

**Errors:**
- `401 Unauthorized` — missing or malformed Authorization header, invalid or expired token, invalid user ID, inactive or missing user, or role mismatch.
- `403 Forbidden` — authenticated user does not have the required role.
- `500 Internal Server Error` — JWT secret is missing or too short, or a database error occurs.

**Usage:** Attach the appropriate guard to a route's parameters to enforce authentication and, where necessary, role-based access.

### `GET /users/me`

Returns the profile of the currently authenticated user.

**Authentication:** Required (`AuthUser`).

**Request body:** None.

**Success response:** `200 OK`

Returns `UserDto` containing the authenticated user's profile information.

**Errors:**
- Authentication errors may occur if the request does not provide valid authentication credentials.

### `POST /listings`

Creates a new lost or found listing for the authenticated user.

**Authentication:** Required (`AuthUser`).

**Request body:** `CreateListingRequest` (JSON)

| Field | Type | Required | Description |
|---|---|---|---|
| `listing_type` | `ListingType` | Yes | Listing type: `lost` or `found` |
| `category_id` | UUID | Yes | ID of the listing category |
| `building_id` | UUID | Yes | ID of the campus building |
| `title` | string | Yes | Listing title; maximum 150 characters |
| `description` | string | Yes | Listing description |
| `location_detail` | string? | No | Additional location details |
| `event_date` | date | Yes | Date of the loss or discovery |

**Success response:** `200 OK`

Returns the created listing as `ListingDto`.

**Errors:**
- `400 Bad Request` — `invalid_title` if the title is empty or exceeds 150 characters; `invalid_description` if the description is empty; `invalid_reference` if the specified category or building does not exist.
- `401 Unauthorized` — missing or invalid authentication.
- `500 Internal Server Error` — `database_error` if the listing could not be created.

**Notes:**
- The listing is associated with the authenticated user.
- New listings are created with the `pending_review` status.
- Image upload is not implemented yet; `photo_path` is currently stored as an empty string.

### `GET /listings?<query..>`

Retrieves a list of listings. Supports filtering by keyword, type, category, building, date range, and status.

**Authentication:** Not required.

**Query parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `q` | string | No | Searches listing titles and descriptions (case-insensitive) |
| `listing_type` | `ListingType` | No | Filter by `lost` or `found` |
| `category_id` | UUID | No | Filter by category |
| `building_id` | UUID | No | Filter by campus building |
| `date_from` | date | No | Include listings from this date onwards |
| `date_to` | date | No | Include listings up to this date |
| `include_archived` | boolean | No | Include archived listings when set to `true` |
| `status` | `ListingStatus` | No | Filter by listing status |

**Success response:** `200 OK`

Returns an array of `ListingDto` objects. An empty array is returned if no listings match the filters.

**Errors:**
- `400 Bad Request` — `invalid_filter` if query parameters are invalid.
- `500 Internal Server Error` — `database_error` if listings could not be retrieved.

**Notes:**
- Pending-review and removed listings are always excluded, even if requested through the `status` filter.
- Archived listings are excluded by default. Set `include_archived=true` to include them.
- Keyword search checks both the title and description.

### `GET /moderation/listings`

Retrieves all listings awaiting moderation, ordered by creation date (oldest first).

**Authentication:** Required (`ModeratorGuard`). Accessible to moderators and admins.

**Request body:** None.

**Success response:** `200 OK`

Returns an array of `ListingDto` objects. An empty array is returned if there are no pending listings.

**Errors:**
- `401 Unauthorized` — missing or invalid authentication.
- `403 Forbidden` — user does not have moderator or admin privileges.
- `500 Internal Server Error` — `database_error` if pending listings could not be retrieved.

### `POST /moderation/listings`

Approves or removes a listing that is awaiting moderation.

**Authentication:** Required (`ModeratorGuard`). Accessible to moderators and admins.

**Request body:** `ModerationActionRequest` (JSON)

| Field | Type | Required | Description |
|---|---|---|---|
| `listing_id` | UUID | Yes | ID of the listing to moderate |
| `action` | `ModerationAction` | Yes | Action to perform: `approve` or `remove` |
| `reason` | string? | Required for `remove` | Reason for removing the listing |

**Success response:** `200 OK`

Returns the updated listing as `ListingDto`.

**Errors:**
- `400 Bad Request` — `rejection_reason_required` if a reason is not provided when removing a listing.
- `401 Unauthorized` — missing or invalid authentication.
- `403 Forbidden` — user does not have moderator or admin privileges.
- `409 Conflict` — `listing_not_pending` if the listing does not exist or is no longer awaiting review.
- `500 Internal Server Error` — `database_error` if moderation fails.

**Notes:**
- Approval changes the listing status to `active`.
- Removal changes the status to `removed`.
- The moderator ID and moderation timestamp are saved.
- Every moderation action is recorded in the listing status history and moderation log.
- The listing owner receives a notification about the result.
- Match suggestions are generated automatically when a listing is approved.
- All changes are performed in a database transaction.

### `GET /users/me/listings`

Retrieves all listings created by the currently authenticated user, ordered by creation date (newest first).

**Authentication:** Required (`AuthUser`).

**Request body:** None.

**Success response:** `200 OK`

Returns an array of `ListingDto` objects. An empty array is returned if the user has no listings.

**Errors:**
- `401 Unauthorized` — missing or invalid authentication.
- `500 Internal Server Error` — `database_error` if the listings could not be retrieved.

### `GET /listings/{listing_id}/matches`

Retrieves match suggestions involving a specific listing, ordered by match score (highest first).

**Authentication:** Required (`AuthUser`). Users can only view matches for their own listings.

**Path parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `listing_id` | UUID | Yes | ID of the listing |

**Request body:** None.

**Success response:** `200 OK`

Returns an array of `MatchDto` objects containing:
- Match ID and the IDs of the lost and found listings.
- Match score.
- Match status.
- Confirmation details (`confirmed_by`, `confirmed_at`).
- Creation timestamp.

An empty array is returned if no matches are found.

**Errors:**
- `400 Bad Request` — `invalid_listing_id` if the ID is not a valid UUID.
- `401 Unauthorized` — missing or invalid authentication.
- `403 Forbidden` — `forbidden` if the listing belongs to another user.
- `404 Not Found` — `listing_not_found` if the listing does not exist.
- `500 Internal Server Error` — `database_error` if the listing or match suggestions could not be retrieved.

### `POST /matches/{match_id}/action`

Confirms or rejects a suggested match between a lost listing and a found listing.

**Authentication:** Required (`AuthUser`). Only the owner of the lost listing can perform this action.

**Path parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `match_id` | UUID | Yes | ID of the match |

**Request body:** `MatchActionRequest` (JSON)

| Field | Type | Required | Description |
|---|---|---|---|
| `action` | `MatchAction` | Yes | Action to perform: `confirm` or `reject` |

**Success response:** `200 OK`

Returns `MatchActionResponse` containing:
- `match_data` — updated match as `MatchDto`.
- `return_id` — ID of the created return record when the match is confirmed; `null` when rejected.

**Errors:**
- `400 Bad Request` — `invalid_match_id` if the ID is not a valid UUID.
- `401 Unauthorized` — missing or invalid authentication.
- `403 Forbidden` — `not_lost_listing_owner` if the user does not own the lost listing.
- `404 Not Found` — `match_not_found` if the match does not exist.
- `409 Conflict` — `match_conflict` if the match is no longer suggested or either listing is no longer active.
- `500 Internal Server Error` — `database_error` if the action could not be processed.

**Notes:**
- Only matches with the `suggested` status can be confirmed or rejected.
- Rejecting a match changes its status to `rejected`. Both listings remain active.
- Confirming a match changes its status to `confirmed` and both listings to `matched`.
- Confirmation creates a pending return record and adds status history entries for both listings.
- Both listing owners receive a notification. Contact information is not included in notification payloads.
- All changes are performed in a database transaction.

### `GET /matches/{match_id}/contacts`

**Auth:** `AuthUser` (Bearer token required).

**Path parameters:**
- `match_id` — match UUID.

**Request body:** None.

**Success response — `200 OK`:** `MatchContactsResponse`
- `match_id` — match UUID.
- `contact` — `MatchContactDto` containing the *other participant's*:
    - `full_name`
    - `email`
    - `phone`
    - `telegram_handle`
    - `vk_handle`
    - `preferred_contact`

**Errors:**
- `400 Bad Request` — `invalid_match_id`: match ID is not a valid UUID.
- `401 Unauthorized` — missing or invalid authentication.
- `403 Forbidden` — `forbidden`: requester is not a participant in the match, or both listings belong to the same user.
- `404 Not Found` — `match_not_found`: match does not exist.
- `409 Conflict` — `match_not_confirmed`: contact details are available only after match confirmation.
- `500 Internal Server Error` — `database_error`: could not retrieve contact information.

**Notes:**
- Either participant can retrieve the other participant's contact details.
- Both lost and found listings are checked to identify the requester.
- Contact details are never returned before the match is confirmed.
- The requester's own contact details are never returned.

### `GET /notifications`

**Auth:** `AuthUser` (Bearer token required).

**Request body:** None.

**Query parameters:** None.

**Success response — `200 OK`:** `NotificationsResponse`
- `notifications` — array of `NotificationDto`, newest first:
    - `id`
    - `user_id`
    - `notification_type`
    - `payload`
    - `is_read`
    - `created_at`
- `unread_counter` — number of unread notifications belonging to the authenticated user.

**Errors:**
- `401 Unauthorized` — missing or invalid authentication.
- `500 Internal Server Error` — `database_error`: could not retrieve notifications.

**Notes:**
- Returns only notifications belonging to the authenticated user.
- Notifications are ordered by `created_at` descending, then `id` descending.
- `unread_counter` counts notifications where `is_read = false`.
- Notifications are not marked as read by this endpoint.
- Returns an empty array and `unread_counter: 0` if the user has no notifications.

### `PATCH /notifications/{notification_id}/read`

**Auth:** `AuthUser` (Bearer token required).

**Path parameters:**
- `notification_id` — notification ID (`i64`).

**Request body:** None.

**Success response — `204 No Content`:** No response body.

**Errors:**
- `401 Unauthorized` — missing or invalid authentication.
- `404 Not Found` — `notification_not_found`: notification does not exist or does not belong to the authenticated user.
- `500 Internal Server Error` — `database_error`: could not update notification.

**Notes:**
- Marks the specified notification as read (`is_read = true`).
- Users can only update their own notifications.
- Calling the endpoint for an already-read notification also returns `204 No Content`.

### `POST /returns/confirm`

**Auth:** `AuthUser` (Bearer token required).

**Request body — `ReturnConfirmationRequest`:**
- `return_id` — return record UUID.
- `confirmation` — which participant is confirming:
    - `loser` — owner of the lost listing.
    - `finder` — owner of the found listing.

**Success response — `200 OK`:** `ReturnDto`

Returns the updated return record, including:
- `loser_confirmed`
- `loser_confirmed_at`
- `finder_confirmed`
- `finder_confirmed_at`
- `final_status`
- `returned_at`

**Errors:**
- `401 Unauthorized` — missing or invalid authentication.
- `403 Forbidden` — `not_a_participant`: requester does not own the listing for the requested confirmation side.
- `404 Not Found` — `return_not_found`: return record does not exist.
- `409 Conflict` — `return_conflict`: return is not pending, match is not confirmed, or this side has already confirmed.
- `500 Internal Server Error` — `database_error`: could not process the return confirmation.

**Notes:**
- The return must be pending and its match must already be confirmed.
- Each participant confirms separately. A participant can confirm only their own side.
- Confirmation is recorded with a timestamp.
- After the first confirmation, the other participant receives a `return_confirmed_by_other` notification.
- Once both participants confirm:
    - Return status becomes `completed`, and `returned_at` is set.
    - Both listings transition from `matched` → `returned` → `archived`.
    - Both status transitions are recorded in listing status history.
    - Both participants receive a `return_completed` notification.
- The operation runs in a database transaction. Row locks serialize concurrent confirmations for the same return.

### `GET /categories`

**Auth:** None (public endpoint).

**Request body:** None.

**Query parameters:** None.

**Success response — `200 OK`:** Array of `CategoryDto`.

Returns only active categories, sorted alphabetically by name.

**Errors:**
- `500 Internal Server Error` — database query failed.

**Notes:**
- Inactive categories (`is_active = false`) are excluded.
- Use category UUIDs from this endpoint when creating listings, rather than category names.
- Returns an empty array if no active categories exist.

### `GET /buildings`

**Auth:** None (public endpoint).

**Request body:** None.

**Query parameters:** None.

**Success response — `200 OK`:** Array of `CampusBuildingDto`.

Returns only active campus buildings, sorted alphabetically by name.

**Errors:**
- `500 Internal Server Error` — database query failed.

**Notes:**
- Inactive buildings (`is_active = false`) are excluded.
- Use building UUIDs from this endpoint when creating listings, rather than building names.
- Returns an empty array if no active buildings exist.