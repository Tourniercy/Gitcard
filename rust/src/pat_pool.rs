//! Pool de jetons PAT (port de `apps/api/src/utils/pat-pool.ts`, phase 3).
//!
//! Round-robin sur `PAT_1..N` (index modulo) ; `markExhausted(token)` →
//! refroidissement d'UNE HEURE (60×60×1000 ms) ; `getNextToken` saute les jetons
//! en cooldown (expiré → sortie de la map, vérif à la date courante) ; tous
//! épuisés → `AllRateLimited`. Pool vide → erreur à la construction (« At least
//! one PAT is required », message du legacy / zod du boot).
//!
//! **Horloge injectable** : aucune lecture de `std::time` dans la logique
//! testée (brief P3 B1) — l'heure arrive par la closure `now_ms`.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Heure courante injectable (ms, même sémantique que `Date.now()`).
pub type NowMs = Arc<dyn Fn() -> u64 + Send + Sync>;

const COOLDOWN_MS: u64 = 60 * 60 * 1000;

#[derive(Debug, Clone, PartialEq)]
pub enum PatPoolError {
    /// `createPatPool([])` → `throw new Error('At least one PAT is required')`.
    Empty,
    /// `getNextToken` avec tous les jetons en cooldown →
    /// `throw new Error('All PAT tokens are rate-limited')`.
    AllRateLimited,
}

impl fmt::Display for PatPoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatPoolError::Empty => write!(f, "At least one PAT is required"),
            PatPoolError::AllRateLimited => write!(f, "All PAT tokens are rate-limited"),
        }
    }
}

pub struct PatPool {
    tokens: Vec<String>,
    index: usize,
    exhausted_until: HashMap<String, u64>,
    now_ms: NowMs,
}

/// `Debug` manuel : la closure `now_ms` (`Arc<dyn Fn>`) n'est pas `Debug`.
impl fmt::Debug for PatPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PatPool")
            .field("tokens", &self.tokens)
            .field("index", &self.index)
            .field("exhausted_until", &self.exhausted_until)
            .finish_non_exhaustive()
    }
}

impl PatPool {
    /// `createPatPool(tokens)` — lève si la liste est vide.
    pub fn new(tokens: Vec<String>, now_ms: NowMs) -> Result<Self, PatPoolError> {
        if tokens.is_empty() {
            return Err(PatPoolError::Empty);
        }
        Ok(Self {
            tokens,
            index: 0,
            exhausted_until: HashMap::new(),
            now_ms,
        })
    }

    fn is_available(&mut self, token: &str) -> bool {
        match self.exhausted_until.get(token) {
            None => true,
            Some(&until) => {
                if (self.now_ms)() >= until {
                    self.exhausted_until.remove(token); // cooldown expiré → sortie de la map
                    true
                } else {
                    false
                }
            }
        }
    }

    /// `getNextToken()` : do/while du legacy — part de `index`, essaie chaque
    /// jeton du cycle, retourne le premier disponible ; un tour complet sans
    /// succès → `AllRateLimited`.
    pub fn get_next_token(&mut self) -> Result<String, PatPoolError> {
        let start_index = self.index;
        loop {
            let token = self.tokens[self.index % self.tokens.len()].clone();
            self.index = (self.index + 1) % self.tokens.len();
            if self.is_available(&token) {
                return Ok(token);
            }
            if self.index == start_index {
                return Err(PatPoolError::AllRateLimited);
            }
        }
    }

    /// `markExhausted(token)` : cooldown d'une heure à partir de maintenant.
    pub fn mark_exhausted(&mut self, token: &str) {
        let until = (self.now_ms)() + COOLDOWN_MS;
        self.exhausted_until.insert(token.to_string(), until);
    }
}

/// Lecture de `PAT_1..PAT_N` (port de `collectPats`, `config.ts`) :
/// séquentiels depuis 1, arrêt au premier absent ou vide ; `PAT_1` absent →
/// liste vide → [`PatPool::new`] rend [`PatPoolError::Empty`] au démarrage
/// (crash du legacy au boot, message zod « At least PAT_1 must be set »).
pub fn collect_pats<F>(mut env: F) -> Vec<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let mut tokens = Vec::new();
    let mut i = 1usize;
    while let Some(pat) = env(&format!("PAT_{i}")) {
        if pat.is_empty() {
            break;
        }
        tokens.push(pat);
        i += 1;
    }
    tokens
}
