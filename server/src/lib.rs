use spacetimedb::{Identity, ReducerContext, Table, Timestamp};

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

#[spacetimedb::table(accessor = user, public)]
pub struct User {
    #[primary_key]
    pub identity: Identity,
    pub display_name: String,
    pub oauth_link: String,
    pub email: String,
    pub online: bool,
    pub created_at: Timestamp,
}

/// Discovered celestial bodies in the galaxy.
#[spacetimedb::table(accessor = discovery, public)]
pub struct Discovery {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub discoverer: Identity,
    pub name: String,
    pub body_type: String,
    pub description: String,
    /// Distance from Earth in light-years
    pub distance_ly: u64,
    pub created_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    log::info!("Galaxy module initialized");
}

#[spacetimedb::reducer(client_connected)]
pub fn on_connect(ctx: &ReducerContext) {
    let caller = ctx.sender();
    if let Some(u) = ctx.db.user().identity().find(&caller) {
        ctx.db.user().identity().update(User { online: true, ..u });
    } else {
        ctx.db.user().insert(User {
            identity: caller,
            display_name: format!("Explorer-{}", &caller.to_hex()[..8]),
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
// Discovery reducers
// ---------------------------------------------------------------------------

#[spacetimedb::reducer]
pub fn log_discovery(
    ctx: &ReducerContext,
    name: String,
    body_type: String,
    description: String,
    distance_ly: u64,
) {
    let caller = ctx.sender();
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        log::error!("Name must be 1-100 characters");
        return;
    }
    let valid_types = ["star", "planet", "nebula", "galaxy", "black hole", "asteroid", "comet", "other"];
    if !valid_types.contains(&body_type.as_str()) {
        log::error!("Invalid body type");
        return;
    }
    if description.len() > 5000 {
        log::error!("Description too long");
        return;
    }
    ctx.db.discovery().insert(Discovery {
        id: 0,
        discoverer: caller,
        name,
        body_type,
        description,
        distance_ly,
        created_at: ctx.timestamp,
    });
}

#[spacetimedb::reducer]
pub fn delete_discovery(ctx: &ReducerContext, discovery_id: u64) {
    let caller = ctx.sender();
    if let Some(d) = ctx.db.discovery().id().find(&discovery_id) {
        if d.discoverer != caller {
            log::error!("Only the discoverer can delete");
            return;
        }
        ctx.db.discovery().id().delete(&discovery_id);
    }
}
