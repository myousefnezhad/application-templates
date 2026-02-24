use adk_rust::{
    anyhow,
    prelude::*,
    session::{
        CreateRequest, DeleteRequest, Event, Events, GetRequest, KEY_PREFIX_APP, KEY_PREFIX_TEMP,
        KEY_PREFIX_USER, ListRequest, Session, SessionService, State,
    },
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::pin::Pin;
use uuid::Uuid;

// ---------- Postgres SessionService ----------
// Matches ADK semantics: create/get/list/delete/append_event  [oai_citation:2‡Docs.rs](https://docs.rs/adk-rust/latest/adk_rust/session/trait.SessionService.html)

type StateMap = HashMap<String, Value>;

pub struct PgSessionService {
    pool: PgPool,
}

impl PgSessionService {
    pub async fn new(pool: PgPool) -> anyhow::Result<Self> {
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE SCHEMA IF NOT EXISTS adk;
            "#,
        )
        .execute(&self.pool)
        .await?;
        // We enforce UNIQUE(session_id) so append_event(session_id, ...) can find the row.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS adk.sessions (
                app_name    TEXT NOT NULL,
                user_id     TEXT NOT NULL,
                session_id  TEXT NOT NULL,
                state       JSONB NOT NULL,
                created_at  TIMESTAMPTZ NOT NULL,
                updated_at  TIMESTAMPTZ NOT NULL,
                PRIMARY KEY (app_name, user_id, session_id),
                UNIQUE (session_id)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS adk.events (
                id          TEXT NOT NULL,
                session_id  TEXT NOT NULL,
                invocation_id TEXT NOT NULL,
                branch      TEXT NOT NULL,
                author      TEXT NOT NULL,
                ts          TIMESTAMPTZ NOT NULL,
                llm_response JSONB NOT NULL,
                actions     JSONB NOT NULL,
                long_running_tool_ids JSONB NOT NULL,
                PRIMARY KEY (id, session_id),
                FOREIGN KEY (session_id) REFERENCES adk.sessions(session_id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS adk.app_states (
                app_name   TEXT PRIMARY KEY,
                state      JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS adk.user_states (
                app_name   TEXT NOT NULL,
                user_id    TEXT NOT NULL,
                state      JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                PRIMARY KEY (app_name, user_id)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    fn extract_state_deltas(delta: &StateMap) -> (StateMap, StateMap, StateMap) {
        let mut app_delta = HashMap::new();
        let mut user_delta = HashMap::new();
        let mut session_delta = HashMap::new();

        for (k, v) in delta {
            if let Some(clean) = k.strip_prefix(KEY_PREFIX_APP) {
                app_delta.insert(clean.to_string(), v.clone());
            } else if let Some(clean) = k.strip_prefix(KEY_PREFIX_USER) {
                user_delta.insert(clean.to_string(), v.clone());
            } else if !k.starts_with(KEY_PREFIX_TEMP) {
                session_delta.insert(k.clone(), v.clone());
            }
        }
        (app_delta, user_delta, session_delta)
    }

    fn merge_states(app: &StateMap, user: &StateMap, session: &StateMap) -> StateMap {
        let mut merged = session.clone();
        for (k, v) in app {
            merged.insert(format!("{KEY_PREFIX_APP}{k}"), v.clone());
        }
        for (k, v) in user {
            merged.insert(format!("{KEY_PREFIX_USER}{k}"), v.clone());
        }
        merged
    }
}

impl SessionService for PgSessionService {
    fn create<'life0, 'async_trait>(
        &'life0 self,
        req: CreateRequest,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = adk_core::Result<Box<dyn Session>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let session_id = req.session_id.unwrap_or_else(|| Uuid::new_v4().to_string());
            let now = Utc::now();

            let (app_delta, user_delta, session_state) = Self::extract_state_deltas(&req.state);

            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| adk_core::AdkError::Session(format!("transaction failed: {e}")))?;

            // app state
            let app_state: StateMap =
                sqlx::query("SELECT state FROM adk.app_states WHERE app_name = $1")
                    .bind(&req.app_name)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| adk_core::AdkError::Session(format!("query failed: {e}")))?
                    .map(|r| r.get::<serde_json::Value, _>("state"))
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

            let mut new_app_state = app_state.clone();
            new_app_state.extend(app_delta);

            sqlx::query(
                r#"
                INSERT INTO adk.app_states(app_name, state, updated_at)
                VALUES ($1, $2, $3)
                ON CONFLICT (app_name) DO UPDATE SET
                    state = EXCLUDED.state,
                    updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind(&req.app_name)
            .bind(
                serde_json::to_value(&new_app_state)
                    .map_err(|e| adk_core::AdkError::Session(format!("serialize failed: {e}")))?,
            )
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| adk_core::AdkError::Session(format!("upsert failed: {e}")))?;

            // user state
            let user_state: StateMap = sqlx::query(
                "SELECT state FROM adk.user_states WHERE app_name = $1 AND user_id = $2",
            )
            .bind(&req.app_name)
            .bind(&req.user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| adk_core::AdkError::Session(format!("query failed: {e}")))?
            .map(|r| r.get::<serde_json::Value, _>("state"))
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

            let mut new_user_state = user_state.clone();
            new_user_state.extend(user_delta);

            sqlx::query(
                r#"
                INSERT INTO adk.user_states(app_name, user_id, state, updated_at)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (app_name, user_id) DO UPDATE SET
                    state = EXCLUDED.state,
                    updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind(&req.app_name)
            .bind(&req.user_id)
            .bind(
                serde_json::to_value(&new_user_state)
                    .map_err(|e| adk_core::AdkError::Session(format!("serialize failed: {e}")))?,
            )
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| adk_core::AdkError::Session(format!("upsert failed: {e}")))?;

            // session state (merged)
            let merged_state = Self::merge_states(&new_app_state, &new_user_state, &session_state);

            sqlx::query(
                r#"
                INSERT INTO adk.sessions(app_name, user_id, session_id, state, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (app_name, user_id, session_id) DO UPDATE SET
                    state = EXCLUDED.state,
                    updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind(&req.app_name)
            .bind(&req.user_id)
            .bind(&session_id)
            .bind(
                serde_json::to_value(&merged_state)
                    .map_err(|e| adk_core::AdkError::Session(format!("serialize failed: {e}")))?,
            )
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| adk_core::AdkError::Session(format!("upsert failed: {e}")))?;

            tx.commit()
                .await
                .map_err(|e| adk_core::AdkError::Session(format!("commit failed: {e}")))?;

            Ok(Box::new(PgSession {
                app_name: req.app_name,
                user_id: req.user_id,
                session_id,
                state: merged_state,
                events: Vec::new(),
                updated_at: now,
            }) as Box<dyn Session>)
        })
    }

    fn get<'life0, 'async_trait>(
        &'life0 self,
        req: GetRequest,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = adk_core::Result<Box<dyn Session>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let row = sqlx::query(
                "SELECT state, updated_at FROM adk.sessions WHERE app_name=$1 AND user_id=$2 AND session_id=$3",
            )
            .bind(&req.app_name)
            .bind(&req.user_id)
            .bind(&req.session_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| adk_core::AdkError::Session(format!("query failed: {e}")))?
            .ok_or_else(|| adk_core::AdkError::Session("session not found".into()))?;

            let state_val: serde_json::Value = row.get("state");
            let state: StateMap = serde_json::from_value(state_val)
                .map_err(|e| adk_core::AdkError::Session(format!("deserialize failed: {e}")))?;

            let updated_at: DateTime<Utc> = row.get("updated_at");

            // events
            let mut events: Vec<Event> = sqlx::query(
                "SELECT llm_response, actions, long_running_tool_ids, id, invocation_id, branch, author, ts FROM adk.events WHERE session_id=$1 ORDER BY ts",
            )
            .bind(&req.session_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| adk_core::AdkError::Session(format!("events query failed: {e}")))?
            .into_iter()
            .filter_map(|r| {
                let llm_response: serde_json::Value = r.get("llm_response");
                let actions: serde_json::Value = r.get("actions");
                let tool_ids: serde_json::Value = r.get("long_running_tool_ids");

                Some(Event {
                    id: r.get("id"),
                    timestamp: r.get("ts"),
                    invocation_id: r.get("invocation_id"),
                    invocation_id_camel: r.get("invocation_id"),
                    branch: r.get("branch"),
                    author: r.get("author"),
                    llm_request: None,
                    llm_response: serde_json::from_value(llm_response).ok()?,
                    actions: serde_json::from_value(actions).ok()?,
                    long_running_tool_ids: serde_json::from_value(tool_ids).ok()?,
                    gcp_llm_request: None,
                    gcp_llm_response: None,
                })
            })
            .collect();

            if let Some(num) = req.num_recent_events {
                let start = events.len().saturating_sub(num);
                events = events[start..].to_vec();
            }
            if let Some(after) = req.after {
                events.retain(|e| e.timestamp >= after);
            }

            Ok(Box::new(PgSession {
                app_name: req.app_name,
                user_id: req.user_id,
                session_id: req.session_id,
                state,
                events,
                updated_at,
            }) as Box<dyn Session>)
        })
    }

    fn list<'life0, 'async_trait>(
        &'life0 self,
        req: ListRequest,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<Vec<Box<dyn Session>>>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT session_id, state, updated_at FROM adk.sessions WHERE app_name=$1 AND user_id=$2",
            )
            .bind(&req.app_name)
            .bind(&req.user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| adk_core::AdkError::Session(format!("query failed: {e}")))?;

            let mut out: Vec<Box<dyn Session>> = Vec::new();
            for r in rows {
                let state_val: serde_json::Value = r.get("state");
                let state: StateMap = serde_json::from_value(state_val).unwrap_or_default();
                let updated_at: DateTime<Utc> = r.get("updated_at");
                let sid: String = r.get("session_id");

                out.push(Box::new(PgSession {
                    app_name: req.app_name.clone(),
                    user_id: req.user_id.clone(),
                    session_id: sid,
                    state,
                    events: Vec::new(),
                    updated_at,
                }) as Box<dyn Session>);
            }
            Ok(out)
        })
    }

    fn delete<'life0, 'async_trait>(
        &'life0 self,
        req: DeleteRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            sqlx::query(
                "DELETE FROM adk.sessions WHERE app_name=$1 AND user_id=$2 AND session_id=$3",
            )
            .bind(&req.app_name)
            .bind(&req.user_id)
            .bind(&req.session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| adk_core::AdkError::Session(format!("delete failed: {e}")))?;
            Ok(())
        })
    }

    fn append_event<'life0, 'life1, 'async_trait>(
        &'life0 self,
        session_id: &'life1 str,
        mut event: Event,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            // same semantic: ignore temp keys  [oai_citation:3‡Docs.rs](https://docs.rs/adk-session/0.2.1/x86_64-unknown-linux-gnu/src/adk_session/inmemory.rs.html)
            event
                .actions
                .state_delta
                .retain(|k, _| !k.starts_with(KEY_PREFIX_TEMP));

            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| adk_core::AdkError::Session(format!("transaction failed: {e}")))?;

            let row = sqlx::query(
                "SELECT app_name, user_id, state FROM adk.sessions WHERE session_id=$1",
            )
            .bind(session_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| adk_core::AdkError::Session(format!("query failed: {e}")))?
            .ok_or_else(|| adk_core::AdkError::Session("session not found".into()))?;

            let app_name: String = row.get("app_name");
            let user_id: String = row.get("user_id");

            let mut state: StateMap =
                serde_json::from_value::<StateMap>(row.get::<serde_json::Value, _>("state"))
                    .map_err(|e| adk_core::AdkError::Session(format!("deserialize failed: {e}")))?;

            let (app_delta, user_delta, session_delta) =
                Self::extract_state_deltas(&event.actions.state_delta);
            state.extend(session_delta);

            // write event
            sqlx::query(
                r#"
                INSERT INTO adk.events(
                    id, session_id, invocation_id, branch, author, ts,
                    llm_response, actions, long_running_tool_ids
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                ON CONFLICT (id, session_id) DO UPDATE SET
                    invocation_id = EXCLUDED.invocation_id,
                    branch = EXCLUDED.branch,
                    author = EXCLUDED.author,
                    ts = EXCLUDED.ts,
                    llm_response = EXCLUDED.llm_response,
                    actions = EXCLUDED.actions,
                    long_running_tool_ids = EXCLUDED.long_running_tool_ids
                "#,
            )
            .bind(&event.id)
            .bind(session_id)
            .bind(&event.invocation_id)
            .bind(&event.branch)
            .bind(&event.author)
            .bind(event.timestamp)
            .bind(serde_json::to_value(&event.llm_response)?)
            .bind(serde_json::to_value(&event.actions)?)
            .bind(serde_json::to_value(&event.long_running_tool_ids)?)
            .execute(&mut *tx)
            .await
            .map_err(|e| adk_core::AdkError::Session(format!("insert event failed: {e}")))?;

            // persist app/user state deltas
            if !app_delta.is_empty() {
                let current: StateMap =
                    sqlx::query("SELECT state FROM adk.app_states WHERE app_name=$1")
                        .bind(&app_name)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(|e| adk_core::AdkError::Session(format!("query failed: {e}")))?
                        .map(|r| r.get::<serde_json::Value, _>("state"))
                        .and_then(|v| serde_json::from_value(v).ok())
                        .unwrap_or_default();

                let mut merged = current;
                merged.extend(app_delta);

                sqlx::query(
                    r#"
                    INSERT INTO adk.app_states(app_name, state, updated_at)
                    VALUES ($1,$2,$3)
                    ON CONFLICT (app_name) DO UPDATE SET state=EXCLUDED.state, updated_at=EXCLUDED.updated_at
                    "#,
                )
                .bind(&app_name)
                .bind(serde_json::to_value(&merged).map_err(|e| adk_core::AdkError::Session(format!("serialize failed: {e}")))?)
                .bind(event.timestamp)
                .execute(&mut *tx)
                .await
                .map_err(|e| adk_core::AdkError::Session(format!("upsert failed: {e}")))?;
            }

            if !user_delta.is_empty() {
                let current: StateMap = sqlx::query(
                    "SELECT state FROM adk.user_states WHERE app_name=$1 AND user_id=$2",
                )
                .bind(&app_name)
                .bind(&user_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| adk_core::AdkError::Session(format!("query failed: {e}")))?
                .map(|r| r.get::<serde_json::Value, _>("state"))
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();

                let mut merged = current;
                merged.extend(user_delta);

                sqlx::query(
                    r#"
                    INSERT INTO adk.user_states(app_name, user_id, state, updated_at)
                    VALUES ($1,$2,$3,$4)
                    ON CONFLICT (app_name, user_id) DO UPDATE SET state=EXCLUDED.state, updated_at=EXCLUDED.updated_at
                    "#,
                )
                .bind(&app_name)
                .bind(&user_id)
                .bind(serde_json::to_value(&merged).map_err(|e| adk_core::AdkError::Session(format!("serialize failed: {e}")))?)
                .bind(event.timestamp)
                .execute(&mut *tx)
                .await
                .map_err(|e| adk_core::AdkError::Session(format!("upsert failed: {e}")))?;
            }

            // update session row
            sqlx::query("UPDATE adk.sessions SET state=$1, updated_at=$2 WHERE session_id=$3")
                .bind(
                    serde_json::to_value(&state).map_err(|e| {
                        adk_core::AdkError::Session(format!("serialize failed: {e}"))
                    })?,
                )
                .bind(event.timestamp)
                .bind(session_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| adk_core::AdkError::Session(format!("update session failed: {e}")))?;

            tx.commit()
                .await
                .map_err(|e| adk_core::AdkError::Session(format!("commit failed: {e}")))?;

            Ok(())
        })
    }
}

struct PgSession {
    app_name: String,
    user_id: String,
    session_id: String,
    state: StateMap,
    events: Vec<Event>,
    updated_at: DateTime<Utc>,
}

impl Session for PgSession {
    fn id(&self) -> &str {
        &self.session_id
    }
    fn app_name(&self) -> &str {
        &self.app_name
    }
    fn user_id(&self) -> &str {
        &self.user_id
    }
    fn state(&self) -> &dyn State {
        self
    }
    fn events(&self) -> &dyn Events {
        self
    }
    fn last_update_time(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

impl State for PgSession {
    fn get(&self, key: &str) -> Option<Value> {
        self.state.get(key).cloned()
    }
    fn set(&mut self, key: String, value: Value) {
        self.state.insert(key, value);
    }
    fn all(&self) -> HashMap<String, Value> {
        self.state.clone()
    }
}

impl Events for PgSession {
    fn all(&self) -> Vec<Event> {
        self.events.clone()
    }

    fn len(&self) -> usize {
        self.events.len()
    }

    fn at(&self, idx: usize) -> Option<&Event> {
        self.events.get(idx)
    }
}
