use spacetimedb::{Identity, ReducerContext, Table, Timestamp};

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

/// User profile — created automatically on first authenticated request.
/// The `identity` field comes from SpacetimeDB's built-in auth (JWT token).
#[spacetimedb::table(accessor = user, public)]
pub struct User {
    #[primary_key]
    pub identity: Identity,
    pub display_name: String,
    /// Optional: store OAuth provider + subject ID for social login linking.
    /// e.g. "google:118234567890" or "github:12345"
    pub oauth_link: String,
    pub email: String,
    pub online: bool,
    pub created_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    log::info!("Module initialized");
}

#[spacetimedb::reducer(client_connected)]
pub fn on_connect(ctx: &ReducerContext) {
    let caller = ctx.sender();
    if let Some(u) = ctx.db.user().identity().find(&caller) {
        ctx.db.user().identity().update(User { online: true, ..u });
    } else {
        ctx.db.user().insert(User {
            identity: caller,
            display_name: format!("User-{}", &caller.to_hex()[..8]),
            oauth_link: String::new(),
            email: String::new(),
            online: true,
            created_at: ctx.timestamp,
        });
    }
}

#[spacetimedb::reducer(client_disconnected)]
pub fn on_disconnect(ctx: &ReducerContext) {
    let caller = ctx.sender();
    if let Some(u) = ctx.db.user().identity().find(&caller) {
        ctx.db.user().identity().update(User { online: false, ..u });
    }
}

// ---------------------------------------------------------------------------
// Auth reducers
// ---------------------------------------------------------------------------

/// Link an OAuth provider to this identity.
/// Call this after client-side OAuth flow completes.
/// provider_id format: "google:118234567890" or "github:12345"
#[spacetimedb::reducer]
pub fn link_oauth(
    ctx: &ReducerContext,
    provider_id: String,
    email: String,
    display_name: String,
) {
    let caller = ctx.sender();

    if provider_id.is_empty() || provider_id.len() > 200 {
        log::error!("Provider ID must be 1-200 characters");
        return;
    }
    if email.len() > 320 {
        log::error!("Email too long");
        return;
    }
    let display_name = display_name.trim().to_string();
    if display_name.is_empty() || display_name.len() > 50 {
        log::error!("Display name must be 1-50 characters");
        return;
    }

    if let Some(u) = ctx.db.user().identity().find(&caller) {
        ctx.db.user().identity().update(User {
            oauth_link: provider_id,
            email,
            display_name,
            ..u
        });
    }
}

/// Update display name.
#[spacetimedb::reducer]
pub fn update_display_name(ctx: &ReducerContext, new_name: String) {
    let caller = ctx.sender();
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() || new_name.len() > 50 {
        log::error!("Display name must be 1-50 characters");
        return;
    }
    if let Some(u) = ctx.db.user().identity().find(&caller) {
        ctx.db.user().identity().update(User {
            display_name: new_name,
            ..u
        });
    }
}

// ---------------------------------------------------------------------------
// Example: Items (replace with your domain)
// ---------------------------------------------------------------------------

#[spacetimedb::table(accessor = item, public)]
pub struct Item {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub owner: Identity,
    pub title: String,
    pub description: String,
    pub created_at: Timestamp,
}

#[spacetimedb::reducer]
pub fn create_item(ctx: &ReducerContext, title: String, description: String) {
    let caller = ctx.sender();
    let title = title.trim().to_string();
    if title.is_empty() || title.len() > 200 {
        log::error!("Title must be 1-200 characters");
        return;
    }
    if description.len() > 10000 {
        log::error!("Description too long");
        return;
    }
    ctx.db.item().insert(Item {
        id: 0,
        owner: caller,
        title,
        description,
        created_at: ctx.timestamp,
    });
}

#[spacetimedb::reducer]
pub fn delete_item(ctx: &ReducerContext, item_id: u64) {
    let caller = ctx.sender();
    if let Some(item) = ctx.db.item().id().find(&item_id) {
        if item.owner != caller {
            log::error!("Only the owner can delete");
            return;
        }
        ctx.db.item().id().delete(&item_id);
    }
}
