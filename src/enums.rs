use crate::models::{
    DBContactMethod, DBListingStatus, DBListingType, DBMatchStatus, DBModerationAction,
    DBNotificationType, DBReturnStatus, DBUserRole, User,
};
use campuslost_shared::{
    ContactMethod, ListingStatus, ListingType, MatchStatus, ModerationAction,
    NotificationType, ReturnStatus, UserDto, UserRole,
};

macro_rules! enum_mappers {
    ($db:ty, $shared:ty, { $($variant:ident),+ $(,)? }) => {
        impl From<$db> for $shared {
            fn from(value: $db) -> Self {
                match value { $( <$db>::$variant => <$shared>::$variant, )+ }
            }
        }

        impl From<$shared> for $db {
            fn from(value: $shared) -> Self {
                match value { $( <$shared>::$variant => <$db>::$variant, )+ }
            }
        }
    };
}

enum_mappers!(DBUserRole, UserRole, { Student, Moderator, Admin });
enum_mappers!(DBContactMethod, ContactMethod, { Email, Phone, Telegram, Vk });
enum_mappers!(DBListingType, ListingType, { Lost, Found });
enum_mappers!(DBListingStatus, ListingStatus, { PendingReview, Active, Matched, Returned, Removed, Archived });
enum_mappers!(DBMatchStatus, MatchStatus, { Suggested, Confirmed, Rejected });
enum_mappers!(DBReturnStatus, ReturnStatus, { Pending, Completed, Cancelled });
enum_mappers!(DBModerationAction, ModerationAction, { Approve, Remove });
enum_mappers!(DBNotificationType, NotificationType, {
    ListingCreated, ListingApproved, ListingRemoved, MatchFound, MatchConfirmed,
    ReturnConfirmedByOther, ReturnCompleted, ReviewReceived
});