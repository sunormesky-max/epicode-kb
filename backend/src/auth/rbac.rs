//! RBAC decision engine.

use crate::auth::model::{GlobalRole, Permission, SpaceRole};
use crate::error::{AppError, AppResult};

/// Context required to evaluate permissions.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub global_role: GlobalRole,
    pub space_id: String,
    pub space_role: Option<SpaceRole>,
}

/// RBAC engine.
pub struct RbacEngine;

impl RbacEngine {
    /// Create a new RBAC engine.
    pub fn new() -> Self {
        Self
    }

    /// Check whether the actor has the requested permission in the given space.
    pub fn check(&self, ctx: &AuthContext, permission: Permission) -> AppResult<()> {
        if Self::has_permission(ctx.global_role, ctx.space_role, &permission) {
            Ok(())
        } else {
            Err(AppError::forbidden(format!(
                "user {} lacks permission {:?}",
                ctx.user_id, permission
            )))
        }
    }

    /// Determine whether a memory is readable by the actor based on space/memory visibility.
    pub fn can_read_memory(
        &self,
        ctx: &AuthContext,
        memory_author_id: Option<&str>,
        memory_visibility: crate::memory::model::Visibility,
        space_visibility: crate::auth::model::SpaceVisibility,
        explicit_permissions: &[crate::auth::model::MemoryPermission],
    ) -> bool {
        use crate::auth::model::{SpaceRole as SR, SpaceVisibility as SV};
        use crate::memory::model::Visibility as MV;

        // Global admins can read everything.
        if ctx.global_role == GlobalRole::Admin {
            return true;
        }

        let is_space_member = ctx.space_role.is_some();
        let is_space_owner = ctx.space_role == Some(SR::Owner);
        let is_author = memory_author_id
            .map(|id| id == ctx.user_id)
            .unwrap_or(false);

        // Space visibility check.
        let space_readable = match space_visibility {
            SV::Public => true,
            SV::Team | SV::Private => is_space_member,
        };

        // Memory visibility check.
        match memory_visibility {
            MV::Inherit => space_readable,
            MV::SpaceOnly => is_space_member,
            MV::Private => is_author || is_space_owner,
            MV::Selected => {
                is_space_owner
                    || explicit_permissions
                        .iter()
                        .any(|p| p.user_id == ctx.user_id && p.permission == "read")
            }
        }
    }

    fn has_permission(
        global_role: GlobalRole,
        space_role: Option<SpaceRole>,
        permission: &Permission,
    ) -> bool {
        // Global role permissions.
        let global_ok = match global_role {
            GlobalRole::Admin => true,
            GlobalRole::Owner => !matches!(permission, Permission::UserManage),
            GlobalRole::Editor => matches!(
                permission,
                Permission::SpaceRead
                    | Permission::SpaceWrite
                    | Permission::MemoryRead
                    | Permission::MemoryWrite
            ),
            GlobalRole::Viewer => {
                matches!(permission, Permission::SpaceRead | Permission::MemoryRead)
            }
        };

        if global_ok {
            return true;
        }

        // Space role fallback.
        if let Some(role) = space_role {
            match role {
                SpaceRole::Owner => true,
                SpaceRole::Editor => matches!(
                    permission,
                    Permission::SpaceRead
                        | Permission::SpaceWrite
                        | Permission::MemoryRead
                        | Permission::MemoryWrite
                        | Permission::AgentWrite
                        | Permission::ApiKeyManage
                ),
                SpaceRole::Viewer => {
                    matches!(permission, Permission::SpaceRead | Permission::MemoryRead)
                }
            }
        } else {
            false
        }
    }
}

impl Default for RbacEngine {
    fn default() -> Self {
        Self::new()
    }
}
