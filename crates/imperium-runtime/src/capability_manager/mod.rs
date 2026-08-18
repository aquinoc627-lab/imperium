//! Capability Manager
//! 
//! Manages capability tokens, validation, and enforcement.

use crate::sandbox::Sandbox;
use imperium_core::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CapabilityManager {
    registry: Arc<RwLock<CapabilityRegistry>>,
    token_store: Arc<RwLock<TokenStore>>,
    sandbox: Sandbox,
}

struct CapabilityRegistry {
    capabilities: HashMap<CapabilityId, RegistryEntry>,
}

struct TokenStore {
    tokens: HashMap<TokenId, CapabilityToken>,
    nonces: HashMap<[u8; 32], TokenId>,
}

impl CapabilityManager {
    pub fn new(sandbox: Sandbox) -> Self {
        Self {
            registry: Arc::new(RwLock::new(CapabilityRegistry { capabilities: HashMap::new() })),
            token_store: Arc::new(RwLock::new(TokenStore { tokens: HashMap::new(), nonces: HashMap::new() })),
            sandbox,
        }
    }
    
    pub async fn register(&self, entry: RegistryEntry) -> Result<CapabilityId> {
        let mut registry = self.registry.write().await;
        registry.capabilities.insert(entry.manifest.id, entry);
        Ok(entry.manifest.id)
    }
    
    pub async fn get(&self, id: CapabilityId) -> Option<RegistryEntry> {
        self.registry.read().await.capabilities.get(&id).cloned()
    }
    
    pub async fn issue_token(
        &self,
        capability_id: CapabilityId,
        subject: ActorId,
        permissions: GrantedPermissions,
        ttl: chrono::Duration,
    ) -> Result<CapabilityToken> {
        // Validate capability exists and permissions are subset
        let capability = self.get(capability_id).await
            .ok_or_else(|| anyhow::anyhow!("Capability not found"))?;
        
        // Validate permissions subset
        Self::validate_permissions(&permissions, &capability.manifest.capabilities)?;
        
        // Generate token
        let token = CapabilityToken {
            id: TokenId::new(),
            capability_id,
            permissions,
            subject,
            issuer: ActorId::new(), // Policy engine
            issued_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + ttl,
            nonce: rand::random(),
            signature: vec![], // TODO: Sign
        };
        
        // Store token
        let mut store = self.token_store.write().await;
        store.tokens.insert(token.id, token.clone());
        store.nonces.insert(token.nonce, token.id);
        
        Ok(token)
    }
    
    pub async fn validate_token(&self, token: &CapabilityToken) -> Result<()> {
        let store = self.token_store.read().await;
        
        // Check exists
        let stored = store.tokens.get(&token.id)
            .ok_or_else(|| anyhow::anyhow!("Token not found"))?;
        
        // Check signature
        // TODO: Verify signature
        
        // Check expiration
        if chrono::Utc::now() > token.expires_at {
            return Err(anyhow::anyhow!("Token expired"));
        }
        
        // Check nonce not reused
        if let Some(existing_id) = store.nonces.get(&token.nonce) {
            if *existing_id != token.id {
                return Err(anyhow::anyhow!("Nonce reused"));
            }
        }
        
        // Check capability still exists
        let capability = self.get(token.capability_id).await
            .ok_or_else(|| anyhow::anyhow!("Capability no longer available"))?;
        
        // Check permissions still valid
        Self::validate_permissions(&token.permissions, &capability.manifest.capabilities)?;
        
        Ok(())
    }
    
    fn validate_permissions(granted: &GrantedPermissions, declared: &DeclaredCapabilities) -> Result<()> {
        // Check each granted permission is subset of declared
        // TODO: Implement full validation
        Ok(())
    }
    
    pub async fn revoke_token(&self, token_id: TokenId) -> Result<()> {
        let mut store = self.token_store.write().await;
        if let Some(token) = store.tokens.remove(&token_id) {
            store.nonces.remove(&token.nonce);
        }
        Ok(())
    }
}