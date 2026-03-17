use spacetimedb::{Identity, ProcedureContext, ReducerContext, Table, Timestamp};
use serde::Serialize;

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

#[spacetimedb::table(accessor = user, public)]
pub struct User {
    #[primary_key]
    pub identity: Identity,
    pub display_name: String,
    pub email: String,
    pub oauth_link: String,
    /// 0 = user, 1 = admin
    pub role: u8,
    pub online: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Example content table — replace with your domain model.
#[spacetimedb::table(accessor = post, public)]
pub struct Post {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub owner: Identity,
    pub title: String,
    pub body: String,
    /// 0 = draft, 1 = published
    pub status: u8,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Example content table — celestial discoveries.
#[spacetimedb::table(accessor = discovery, public)]
pub struct Discovery {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub owner: Identity,
    pub name: String,
    pub body_type: String,
    pub description: String,
    pub distance_ly: u64,
    pub created_at: Timestamp,
}

/// Rate limiting — tracks last action time per identity per action type.
#[spacetimedb::table(accessor = rate_limit, private)]
pub struct RateLimit {
    #[primary_key]
    pub identity: Identity,
    pub last_action_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check if caller is a registered user. Returns the user row or None.
fn get_user(ctx: &ReducerContext) -> Option<User> {
    ctx.db.user().identity().find(&ctx.sender())
}

/// Check if caller has admin role.
fn is_admin(ctx: &ReducerContext) -> bool {
    ctx.db
        .user()
        .identity()
        .find(&ctx.sender())
        .map_or(false, |u| u.role == 1)
}

/// Simple rate limiter. Returns true if the action is allowed (enough time has passed).
fn check_rate_limit(ctx: &ReducerContext, min_interval_ms: u64) -> bool {
    let caller = ctx.sender();
    let now = ctx.timestamp;
    if let Some(rl) = ctx.db.rate_limit().identity().find(&caller) {
        let elapsed = now
            .duration_since(rl.last_action_at)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(u64::MAX);
        if elapsed < min_interval_ms {
            return false;
        }
        ctx.db.rate_limit().identity().update(RateLimit {
            identity: caller,
            last_action_at: now,
        });
    } else {
        ctx.db.rate_limit().insert(RateLimit {
            identity: caller,
            last_action_at: now,
        });
    }
    true
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
        ctx.db.user().identity().update(User {
            online: true,
            updated_at: ctx.timestamp,
            ..u
        });
    }
    // Don't auto-create users on connect — require explicit registration
}

#[spacetimedb::reducer(client_disconnected)]
pub fn on_disconnect(ctx: &ReducerContext) {
    let caller = ctx.sender();
    if let Some(u) = ctx.db.user().identity().find(&caller) {
        ctx.db.user().identity().update(User {
            online: false,
            updated_at: ctx.timestamp,
            ..u
        });
    }
}

// ---------------------------------------------------------------------------
// Auth reducers
// ---------------------------------------------------------------------------

/// Register a new user. Call this once after creating an identity.
#[spacetimedb::reducer]
pub fn register(ctx: &ReducerContext, display_name: String) {
    let caller = ctx.sender();

    // Prevent duplicate registration
    if ctx.db.user().identity().find(&caller).is_some() {
        log::error!("Already registered");
        return;
    }

    let display_name = display_name.trim().to_string();
    if display_name.is_empty() || display_name.len() > 50 {
        log::error!("Display name must be 1-50 characters");
        return;
    }

    ctx.db.user().insert(User {
        identity: caller,
        display_name,
        email: String::new(),
        oauth_link: String::new(),
        role: 0,
        online: true,
        created_at: ctx.timestamp,
        updated_at: ctx.timestamp,
    });
}

/// Link an OAuth provider to the current identity.
#[spacetimedb::reducer]
pub fn link_oauth(
    ctx: &ReducerContext,
    provider_id: String,
    email: String,
    display_name: String,
) {
    let caller = ctx.sender();

    let user = match get_user(ctx) {
        Some(u) => u,
        None => {
            log::error!("Must register before linking OAuth");
            return;
        }
    };

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

    ctx.db.user().identity().update(User {
        oauth_link: provider_id,
        email,
        display_name,
        updated_at: ctx.timestamp,
        ..user
    });
}

/// Update display name.
#[spacetimedb::reducer]
pub fn update_display_name(ctx: &ReducerContext, new_name: String) {
    let user = match get_user(ctx) {
        Some(u) => u,
        None => {
            log::error!("Not registered");
            return;
        }
    };

    let new_name = new_name.trim().to_string();
    if new_name.is_empty() || new_name.len() > 50 {
        log::error!("Display name must be 1-50 characters");
        return;
    }

    ctx.db.user().identity().update(User {
        display_name: new_name,
        updated_at: ctx.timestamp,
        ..user
    });
}

// ---------------------------------------------------------------------------
// Post reducers — example CRUD with owner checks
// ---------------------------------------------------------------------------

/// Create a new post (rate limited: 1 per 5 seconds).
#[spacetimedb::reducer]
pub fn create_post(ctx: &ReducerContext, title: String, body: String) {
    let caller = ctx.sender();

    // Must be registered
    if get_user(ctx).is_none() {
        log::error!("Must register before creating posts");
        return;
    }

    // Rate limit: 5 seconds between posts
    if !check_rate_limit(ctx, 5000) {
        log::error!("Rate limited — wait before creating another post");
        return;
    }

    let title = title.trim().to_string();
    if title.is_empty() || title.len() > 200 {
        log::error!("Title must be 1-200 characters");
        return;
    }
    if body.len() > 10000 {
        log::error!("Body too long (max 10000 characters)");
        return;
    }

    ctx.db.post().insert(Post {
        id: 0, // auto_inc
        owner: caller,
        title,
        body,
        status: 0, // draft
        created_at: ctx.timestamp,
        updated_at: ctx.timestamp,
    });
}

/// Update a post. Only the owner can update.
#[spacetimedb::reducer]
pub fn update_post(ctx: &ReducerContext, post_id: u64, title: String, body: String) {
    let caller = ctx.sender();

    let post = match ctx.db.post().id().find(&post_id) {
        Some(p) => p,
        None => {
            log::error!("Post not found");
            return;
        }
    };

    // Owner check
    if post.owner != caller {
        log::error!("Only the owner can update this post");
        return;
    }

    let title = title.trim().to_string();
    if title.is_empty() || title.len() > 200 {
        log::error!("Title must be 1-200 characters");
        return;
    }
    if body.len() > 10000 {
        log::error!("Body too long (max 10000 characters)");
        return;
    }

    ctx.db.post().id().update(Post {
        title,
        body,
        updated_at: ctx.timestamp,
        ..post
    });
}

/// Delete a post. Only the owner (or admin) can delete.
#[spacetimedb::reducer]
pub fn delete_post(ctx: &ReducerContext, post_id: u64) {
    let caller = ctx.sender();

    let post = match ctx.db.post().id().find(&post_id) {
        Some(p) => p,
        None => {
            log::error!("Post not found");
            return;
        }
    };

    if post.owner != caller && !is_admin(ctx) {
        log::error!("Only the owner or an admin can delete this post");
        return;
    }

    ctx.db.post().id().delete(&post_id);
}

/// Publish a post (change status from draft to published). Owner only.
#[spacetimedb::reducer]
pub fn publish_post(ctx: &ReducerContext, post_id: u64) {
    let caller = ctx.sender();

    let post = match ctx.db.post().id().find(&post_id) {
        Some(p) => p,
        None => {
            log::error!("Post not found");
            return;
        }
    };

    if post.owner != caller {
        log::error!("Only the owner can publish this post");
        return;
    }

    ctx.db.post().id().update(Post {
        status: 1,
        updated_at: ctx.timestamp,
        ..post
    });
}

// ---------------------------------------------------------------------------
// Admin reducers
// ---------------------------------------------------------------------------

/// Admin: set a user's role. Only admins can call this.
#[spacetimedb::reducer]
pub fn admin_set_role(ctx: &ReducerContext, target_identity: Identity, new_role: u8) {
    if !is_admin(ctx) {
        log::error!("Admin only");
        return;
    }

    if new_role > 1 {
        log::error!("Invalid role (0 = user, 1 = admin)");
        return;
    }

    if let Some(target) = ctx.db.user().identity().find(&target_identity) {
        ctx.db.user().identity().update(User {
            role: new_role,
            updated_at: ctx.timestamp,
            ..target
        });
    } else {
        log::error!("Target user not found");
    }
}

// ---------------------------------------------------------------------------
// Discovery reducers
// ---------------------------------------------------------------------------

/// Log a new celestial discovery (rate limited: 1 per 5 seconds).
#[spacetimedb::reducer]
pub fn log_discovery(
    ctx: &ReducerContext,
    name: String,
    body_type: String,
    description: String,
    distance_ly: u64,
) {
    let caller = ctx.sender();

    if get_user(ctx).is_none() {
        log::error!("Must register before logging discoveries");
        return;
    }

    if !check_rate_limit(ctx, 5000) {
        log::error!("Rate limited — wait before logging another discovery");
        return;
    }

    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        log::error!("Name must be 1-100 characters");
        return;
    }

    let body_type = body_type.trim().to_string();
    let valid_types = [
        "star", "planet", "nebula", "galaxy", "black hole", "asteroid", "comet", "other",
    ];
    if !valid_types.contains(&body_type.as_str()) {
        log::error!("Invalid body type");
        return;
    }

    if description.len() > 5000 {
        log::error!("Description too long (max 5000 characters)");
        return;
    }

    ctx.db.discovery().insert(Discovery {
        id: 0,
        owner: caller,
        name,
        body_type,
        description,
        distance_ly,
        created_at: ctx.timestamp,
    });
}

/// Delete a discovery. Only the owner (or admin) can delete.
#[spacetimedb::reducer]
pub fn delete_discovery(ctx: &ReducerContext, discovery_id: u64) {
    let caller = ctx.sender();

    let disc = match ctx.db.discovery().id().find(&discovery_id) {
        Some(d) => d,
        None => {
            log::error!("Discovery not found");
            return;
        }
    };

    if disc.owner != caller && !is_admin(ctx) {
        log::error!("Only the owner or an admin can delete this discovery");
        return;
    }

    ctx.db.discovery().id().delete(&discovery_id);
}

// ---------------------------------------------------------------------------
// Procedures — return data to the caller (no raw SQL from frontend)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct DiscoveryRow {
    id: u64,
    owner: String,
    name: String,
    body_type: String,
    description: String,
    distance_ly: u64,
}

/// Get all discoveries. No auth required.
#[spacetimedb::procedure]
pub fn get_discoveries(ctx: &mut ProcedureContext) -> String {
    let rows: Vec<DiscoveryRow> = ctx.with_tx(|ctx| {
        ctx.db
            .discovery()
            .iter()
            .map(|d| DiscoveryRow {
                id: d.id,
                owner: hex::encode(d.owner.to_byte_array()),
                name: d.name,
                body_type: d.body_type,
                description: d.description,
                distance_ly: d.distance_ly,
            })
            .collect()
    });
    serde_json::to_string(&rows).unwrap_or_default()
}

#[derive(Serialize)]
struct ProfileInfo {
    display_name: String,
    email: String,
    oauth_link: String,
}

/// Get the caller's profile. Requires auth.
#[spacetimedb::procedure]
pub fn get_my_profile(ctx: &mut ProcedureContext) -> String {
    let caller = ctx.sender();
    let profile = ctx.with_tx(|ctx| {
        ctx.db.user().identity().find(&caller).map(|u| ProfileInfo {
            display_name: u.display_name,
            email: u.email,
            oauth_link: u.oauth_link,
        })
    });
    match profile {
        Some(p) => serde_json::to_string(&p).unwrap_or_default(),
        None => String::from("null"),
    }
}

/// Get a user's display name by identity. No auth required.
#[spacetimedb::procedure]
pub fn get_display_name(ctx: &mut ProcedureContext, identity_hex: String) -> String {
    let bytes = match hex::decode(&identity_hex) {
        Ok(b) => b,
        Err(_) => return String::from("null"),
    };
    let byte_array: [u8; 32] = match bytes.try_into() {
        Ok(a) => a,
        Err(_) => return String::from("null"),
    };
    let identity = Identity::from_byte_array(byte_array);
    let name = ctx.with_tx(|ctx| {
        ctx.db
            .user()
            .identity()
            .find(&identity)
            .map(|u| u.display_name)
    });
    match name {
        Some(n) => serde_json::to_string(&n).unwrap_or_default(),
        None => String::from("null"),
    }
}
