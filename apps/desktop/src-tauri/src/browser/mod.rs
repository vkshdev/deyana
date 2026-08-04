pub mod commands;
pub mod service;
pub mod types;

use std::path::PathBuf;
use crate::db::DbPool;
pub use service::NativeBrowserService;

pub struct BrowserState {
    pub service: NativeBrowserService,
}

impl BrowserState {
    pub fn new(data_dir: PathBuf, db_pool: DbPool) -> Self {
        Self {
            service: NativeBrowserService::new(data_dir, db_pool),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::service::*;
    use super::types::*;

    #[test]
    fn test_validate_browser_url() {
        assert!(validate_browser_url("https://example.com").is_ok());
        assert!(validate_browser_url("http://example.com/test").is_ok());
        assert!(validate_browser_url("ftp://example.com").is_err());
        assert!(validate_browser_url("https://user:pass@example.com").is_err());
    }

    #[test]
    fn test_origin_for_url() {
        assert_eq!(origin_for_url("https://example.com/foo/bar"), "https://example.com");
        assert_eq!(origin_for_url("http://localhost:8080/test"), "http://localhost:8080");
    }

    #[test]
    fn test_normalize_automation_disclosure() {
        assert!(normalize_automation_disclosure("I am Deyana, Vikash's assistant.").is_ok());
        assert!(normalize_automation_disclosure("Hello world").is_err());
    }

    #[test]
    fn test_normalize_busy_template() {
        assert!(normalize_busy_template("I am Deyana, Vikash's assistant. Busy now.").is_ok());
        assert!(normalize_busy_template("I am busy.").is_err());
        assert!(normalize_busy_template("I am Deyana, assistant. Check https://example.com").is_err());
    }

    #[test]
    fn test_infer_mood_label() {
        let (label, conf) = infer_mood_label("This is urgent ASAP!");
        assert_eq!(label, "urgent");
        assert!(conf > 0.7);

        let (label2, _) = infer_mood_label("I am so frustrated and annoyed");
        assert_eq!(label2, "frustrated");
    }

    #[test]
    fn test_classify_busy_message() {
        assert_eq!(classify_busy_message("Your OTP code is 123456"), "otp");
        assert_eq!(classify_busy_message("Send money via UPI payment"), "payment");
        assert_eq!(classify_busy_message("Please respond ASAP"), "urgent");
        assert_eq!(classify_busy_message("Hello how are you"), "normal");
    }

    #[test]
    fn test_classify_browser_voice_intent() {
        assert_eq!(classify_browser_voice_intent("Search the web for news"), "search_web");
        assert_eq!(classify_browser_voice_intent("Open https://example.com"), "open_url");
        assert_eq!(classify_browser_voice_intent("Draft a reply for me"), "draft_reply");
        assert_eq!(classify_browser_voice_intent("Summarize this page"), "summarize_page");
    }
    #[test]
    fn test_native_browser_service_lifecycle() {
        let (dir, service) = create_test_service();
        let status = service.status().unwrap();
        assert_eq!(status.state, "disconnected");
        assert_eq!(status.active_sessions, 0);
        assert_eq!(status.protocol_version, 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_browser_permissions_crud() {
        let (dir, service) = create_test_service();

        let req = BrowserPermissionRequest {
            origin: Some("https://example.com".to_string()),
            kind: "optional_origin".to_string(),
        };
        let resp = service.request_permission(req).unwrap();
        assert_eq!(resp.status, "completed");
        assert!(resp.permission.unwrap().granted);

        let list = service.list_permissions().unwrap();
        assert_eq!(list.total, 1);

        let revoke = service.revoke_permission("https://example.com").unwrap();
        assert_eq!(revoke.status, "completed");

        let list_after = service.list_permissions().unwrap();
        assert_eq!(list_after.total, 0);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_browser_context_reading_and_summary() {
        let (dir, service) = create_test_service();
        let ctx = dummy_page_context();

        let session = service.update_page_context(ctx.clone()).unwrap();
        assert_eq!(session.id, "sess_1");

        let unapproved = service.read_context(BrowserContextReadRequest {
            page_session_id: None,
            origin: None,
            mode: "main".to_string(),
            user_approved: false,
        }).unwrap();
        assert_eq!(unapproved.status, "permission_required");

        let approved = service.read_context(BrowserContextReadRequest {
            page_session_id: None,
            origin: None,
            mode: "main".to_string(),
            user_approved: true,
        }).unwrap();
        assert_eq!(approved.status, "completed");
        assert_eq!(approved.context.unwrap().title, "Test Page");

        let summary = service.summarize_context(BrowserContextSummaryRequest {
            mode: "main".to_string(),
            instruction: "Summarize page".to_string(),
            user_approved: true,
        }).unwrap();
        assert_eq!(summary.status, "completed");
        assert!(summary.summary.contains("Test Page"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_action_plans_lifecycle_and_emergency_stop() {
        let (dir, service) = create_test_service();

        let req = BrowserActionPlanCreateRequest {
            chain: "open_url".to_string(),
            url: Some("https://example.com".to_string()),
            page_session_id: None,
            field_handle: None,
            value: None,
            target_label: None,
            user_approved: true,
        };
        let plan_resp = service.create_action_plan(req).unwrap();
        assert_eq!(plan_resp.status, "completed");
        let plan = plan_resp.plan.unwrap();
        assert_eq!(plan.status, "pending_confirmation");
        let token = plan.confirmation_token.clone().unwrap();

        let confirm_resp = service.confirm_action_plan(BrowserActionConfirmRequest {
            plan_id: plan.id.clone(),
            confirmation_token: token,
        }).unwrap();
        assert_eq!(confirm_resp.plan.unwrap().status, "confirmed");

        let exec_resp = service.execute_action_plan(&plan.id).unwrap();
        assert_eq!(exec_resp.plan.unwrap().status, "completed");

        let stop = service.emergency_stop().unwrap();
        assert!(stop.stopped);

        let policy = service.get_whatsapp_busy_policy().unwrap();
        assert!(policy.emergency_stopped);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_whatsapp_busy_mode_and_evaluation() {
        let (dir, service) = create_test_service();

        let _ = service.patch_whatsapp_busy_policy(WhatsAppBusyModePolicyPatch {
            enabled: Some(true),
            allowlisted_contacts: Some(vec!["Alice".to_string(), "Bob".to_string()]),
            allow_groups: Some(false),
            timezone: None,
            window_start: Some("00:00".to_string()),
            window_end: Some("23:59".to_string()),
            cooldown_minutes: Some(0),
            daily_limit: Some(10),
            template: Some("I am Deyana, Vikash's assistant. Busy right now.".to_string()),
            reset_emergency_stop: Some(true),
        }).unwrap();

        let eval_normal = service.evaluate_whatsapp_busy_mode(WhatsAppBusyModeEvaluationRequest {
            page_session_id: Some("sess_1".to_string()),
            contact_label: Some("Bob".to_string()),
            is_group: Some(false),
            latest_message_text: Some("Hello, are you free?".to_string()),
            user_approved: true,
        }).unwrap();
        assert!(eval_normal.allowed);
        assert_eq!(eval_normal.decision, "allowed");

        let eval_otp = service.evaluate_whatsapp_busy_mode(WhatsAppBusyModeEvaluationRequest {
            page_session_id: Some("sess_1".to_string()),
            contact_label: Some("Bob".to_string()),
            is_group: Some(false),
            latest_message_text: Some("Your OTP code is 987654".to_string()),
            user_approved: true,
        }).unwrap();
        assert!(!eval_otp.allowed);
        assert_eq!(eval_otp.decision, "requires_confirmation");

        let send_resp = service.send_whatsapp_busy_reply(WhatsAppBusyModeSendRequest {
            page_session_id: "sess_1".to_string(),
            field_handle: Some("input_1".to_string()),
            latest_message_text: Some("Are you around?".to_string()),
            user_approved: true,
        }).unwrap();
        assert_eq!(send_resp.status, "completed");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_personality_profile_and_contact_tone() {
        let (dir, service) = create_test_service();

        let patched = service.patch_personality_profile(BrowserPersonalityProfilePatch {
            preset: Some("professional".to_string()),
            display_name: Some("Professional".to_string()),
            custom_instruction: Some("Be concise.".to_string()),
            writer_temperature: Some(0.3),
            max_draft_characters: Some(500),
            automation_disclosure: Some("I am Deyana, Vikash's assistant.".to_string()),
        }).unwrap();
        assert_eq!(patched.preset, "professional");

        let tone = service.save_contact_tone(BrowserContactTonePreferenceRequest {
            adapter_id: "whatsapp".to_string(),
            contact_label: "Boss".to_string(),
            tone_instruction: "Formal and polite".to_string(),
            approved: true,
        }).unwrap();
        assert_eq!(tone.contact_label, "Boss");

        let mood = service.infer_mood(BrowserMoodInferRequest {
            text: "This is urgent ASAP!".to_string(),
            ttl_seconds: Some(300),
        }).unwrap();
        assert_eq!(mood.label, "urgent");

        let prev = service.preview_personality(BrowserPersonalityPreviewRequest {
            sample_text: Some("Will get back soon.".to_string()),
            contact_label: None,
            adapter_id: None,
        }).unwrap();
        assert!(prev.preview.contains("professional"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_voice_command_routing() {
        let (dir, service) = create_test_service();
        let _ = service.update_page_context(dummy_page_context());

        let route_sum = service.route_voice_command(BrowserVoiceCommandRequest {
            transcript: "Summarize this page".to_string(),
            mode: "main".to_string(),
            page_session_id: Some("sess_1".to_string()),
            user_approved: true,
        }).unwrap();
        assert_eq!(route_sum.status, "completed");
        assert_eq!(route_sum.intent, "summarize_page");
        assert!(route_sum.summary.is_some());

        let route_srch = service.route_voice_command(BrowserVoiceCommandRequest {
            transcript: "Search the web for Rust programming".to_string(),
            mode: "main".to_string(),
            page_session_id: None,
            user_approved: true,
        }).unwrap();
        assert_eq!(route_srch.status, "completed");
        assert_eq!(route_srch.intent, "search_web");
        assert!(route_srch.search.is_some());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_empirical_session_limit_and_memory_leak() {
        let (dir, service) = create_test_service();

        for i in 0..10 {
            let mut ctx = dummy_page_context();
            ctx.page_session_id = format!("sess_{}", i);
            ctx.origin = format!("https://site{}.com", i);
            service.update_page_context(ctx).unwrap();
        }

        let list = service.list_sessions().unwrap();
        assert_eq!(list.total, 10);

        let conn = service.conn().unwrap();
        let past = (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        conn.execute("UPDATE browser_sessions SET expires_at = ?1", rusqlite::params![past]).unwrap();

        let list_after = service.list_sessions().unwrap();
        assert_eq!(list_after.total, 0, "DB sessions should be purged after expiry");

        let read = service.read_context(BrowserContextReadRequest {
            page_session_id: Some("sess_0".to_string()),
            origin: None,
            mode: "main".to_string(),
            user_approved: true,
        }).unwrap();

        assert!(read.context.is_none(), "In-memory contexts should be removed after DB purge");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_empirical_read_context_session_routing() {
        let (dir, service) = create_test_service();

        let mut ctx1 = dummy_page_context();
        ctx1.page_session_id = "sess_alpha".to_string();
        ctx1.title = "Alpha Page".to_string();
        service.update_page_context(ctx1).unwrap();

        let mut ctx2 = dummy_page_context();
        ctx2.page_session_id = "sess_beta".to_string();
        ctx2.title = "Beta Page".to_string();
        service.update_page_context(ctx2).unwrap();

        let res_alpha = service.read_context(BrowserContextReadRequest {
            page_session_id: Some("sess_alpha".to_string()),
            origin: None,
            mode: "main".to_string(),
            user_approved: true,
        }).unwrap();
        assert_eq!(res_alpha.context.unwrap().title, "Alpha Page");

        let res_beta = service.read_context(BrowserContextReadRequest {
            page_session_id: Some("sess_beta".to_string()),
            origin: None,
            mode: "main".to_string(),
            user_approved: true,
        }).unwrap();
        assert_eq!(res_beta.context.unwrap().title, "Beta Page");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_empirical_permission_persistence_decoupling() {
        let (dir, service) = create_test_service();

        let ctx = BrowserPageContext {
            origin: "https://secret.com".to_string(),
            ..dummy_page_context()
        };
        service.update_page_context(ctx).unwrap();

        let read = service.read_context(BrowserContextReadRequest {
            page_session_id: Some("sess_1".to_string()),
            origin: Some("https://secret.com".to_string()),
            mode: "main".to_string(),
            user_approved: true,
        }).unwrap();
        assert_eq!(read.status, "completed");

        service.revoke_permission("https://secret.com").unwrap();

        let read_after = service.read_context(BrowserContextReadRequest {
            page_session_id: Some("sess_1".to_string()),
            origin: Some("https://secret.com".to_string()),
            mode: "main".to_string(),
            user_approved: true,
        }).unwrap();
        assert_eq!(read_after.status, "permission_required");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_empirical_action_plan_confirmation_token_validation() {
        let (dir, service) = create_test_service();

        let req = BrowserActionPlanCreateRequest {
            chain: "open_url".to_string(),
            url: Some("https://example.com".to_string()),
            page_session_id: None,
            field_handle: None,
            value: None,
            target_label: None,
            user_approved: true,
        };
        let plan_resp = service.create_action_plan(req).unwrap();
        let plan = plan_resp.plan.unwrap();
        let valid_token = plan.confirmation_token.clone().unwrap();

        let confirm_fail = service.confirm_action_plan(BrowserActionConfirmRequest {
            plan_id: plan.id.clone(),
            confirmation_token: "invalid_wrong_token_12345".to_string(),
        }).unwrap();

        assert_eq!(confirm_fail.status, "failed");

        let confirm_ok = service.confirm_action_plan(BrowserActionConfirmRequest {
            plan_id: plan.id.clone(),
            confirmation_token: valid_token,
        }).unwrap();

        assert_eq!(confirm_ok.status, "completed");
        assert_eq!(confirm_ok.plan.unwrap().status, "confirmed");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_empirical_action_plan_state_machine_edge_cases() {
        let (dir, service) = create_test_service();

        let req = BrowserActionPlanCreateRequest {
            chain: "open_url".to_string(),
            url: Some("https://example.com".to_string()),
            page_session_id: None,
            field_handle: None,
            value: None,
            target_label: None,
            user_approved: true,
        };
        let plan_resp = service.create_action_plan(req).unwrap();
        let plan = plan_resp.plan.unwrap();

        let exec_fail = service.execute_action_plan(&plan.id).unwrap();
        assert_eq!(exec_fail.status, "failed");

        service.confirm_action_plan(BrowserActionConfirmRequest {
            plan_id: plan.id.clone(),
            confirmation_token: plan.confirmation_token.unwrap(),
        }).unwrap();
        let exec_ok = service.execute_action_plan(&plan.id).unwrap();
        assert_eq!(exec_ok.status, "completed");

        let cancel_res = service.cancel_action_plan(&plan.id).unwrap();
        assert_eq!(cancel_res.status, "failed");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_empirical_emergency_stop_scope() {
        let (dir, service) = create_test_service();

        let stop_res = service.emergency_stop().unwrap();
        assert!(stop_res.stopped);

        let eval = service.evaluate_whatsapp_busy_mode(WhatsAppBusyModeEvaluationRequest {
            page_session_id: Some("sess_1".to_string()),
            contact_label: Some("Bob".to_string()),
            is_group: Some(false),
            latest_message_text: Some("Hello".to_string()),
            user_approved: true,
        }).unwrap();
        assert_eq!(eval.decision, "blocked");

        let plan_resp = service.create_action_plan(BrowserActionPlanCreateRequest {
            chain: "open_url".to_string(),
            url: Some("https://example.com".to_string()),
            page_session_id: None,
            field_handle: None,
            value: None,
            target_label: None,
            user_approved: true,
        }).unwrap();
        assert_eq!(plan_resp.status, "completed");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_empirical_whatsapp_policy_enforcement_checks() {
        let (dir, service) = create_test_service();

        let _ = service.patch_whatsapp_busy_policy(WhatsAppBusyModePolicyPatch {
            enabled: Some(true),
            allowlisted_contacts: Some(vec!["Alice".to_string()]),
            allow_groups: Some(false),
            timezone: None,
            window_start: Some("00:00".to_string()),
            window_end: Some("23:59".to_string()),
            cooldown_minutes: Some(60),
            daily_limit: Some(5),
            template: Some("I am Deyana, Vikash's assistant. Busy right now.".to_string()),
            reset_emergency_stop: Some(true),
        }).unwrap();

        let eval_bob = service.evaluate_whatsapp_busy_mode(WhatsAppBusyModeEvaluationRequest {
            page_session_id: Some("sess_1".to_string()),
            contact_label: Some("Bob".to_string()),
            is_group: Some(false),
            latest_message_text: Some("Hey Bob here".to_string()),
            user_approved: true,
        }).unwrap();

        assert!(!eval_bob.allowed, "Bob should be blocked as he is not in the allowlist");
        assert_eq!(eval_bob.decision, "blocked");

        let eval_alice_1 = service.evaluate_whatsapp_busy_mode(WhatsAppBusyModeEvaluationRequest {
            page_session_id: Some("sess_1".to_string()),
            contact_label: Some("Alice".to_string()),
            is_group: Some(false),
            latest_message_text: Some("Hey Alice here".to_string()),
            user_approved: true,
        }).unwrap();
        assert!(eval_alice_1.allowed);
        assert_eq!(eval_alice_1.decision, "allowed");

        let eval_alice_2 = service.evaluate_whatsapp_busy_mode(WhatsAppBusyModeEvaluationRequest {
            page_session_id: Some("sess_1".to_string()),
            contact_label: Some("Alice".to_string()),
            is_group: Some(false),
            latest_message_text: Some("Are you free now?".to_string()),
            user_approved: true,
        }).unwrap();
        assert!(!eval_alice_2.allowed, "Alice should be blocked due to active cooldown");
        assert_eq!(eval_alice_2.decision, "blocked");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_empirical_voice_command_routing_and_extraction() {
        let (dir, service) = create_test_service();
        let _ = service.update_page_context(dummy_page_context());

        let route_unapp = service.route_voice_command(BrowserVoiceCommandRequest {
            transcript: "Summarize this page".to_string(),
            mode: "main".to_string(),
            page_session_id: Some("sess_1".to_string()),
            user_approved: false,
        }).unwrap();
        assert_eq!(route_unapp.status, "permission_required");

        let url_paren = extract_url_from_voice("Open (https://example.com)");
        assert!(url_paren.is_none(), "Leading parenthesis prevents URL extraction");

        let url_clean = extract_url_from_voice("Open https://example.com");
        assert_eq!(url_clean.unwrap(), "https://example.com");

        let _ = std::fs::remove_dir_all(dir);
    }


    fn create_test_service() -> (std::path::PathBuf, NativeBrowserService) {
        let dir = std::env::temp_dir().join(format!("test_browser_{}", uuid::Uuid::new_v4().to_string()));
        let _ = std::fs::create_dir_all(&dir);
        let db_path = dir.join("test.sqlite");
        let pool = crate::db::init_db(&db_path).unwrap();
        let service = NativeBrowserService::new(dir.clone(), pool);
        (dir, service)
    }

    fn dummy_page_context() -> BrowserPageContext {
        BrowserPageContext {
            page_session_id: "sess_1".to_string(),
            origin: "https://example.com".to_string(),
            url: "https://example.com/test".to_string(),
            title: "Test Page".to_string(),
            adapter_id: "generic_page".to_string(),
            adapter_version: 1,
            mode: "main".to_string(),
            captured_at: chrono::Utc::now().to_rfc3339(),
            visible_text: "Test page content visible text.".to_string(),
            selection_text: None,
            main_text: Some("Main test page body content.".to_string()),
            landmarks: vec![],
            available_actions: vec![],
            writable_fields: vec![BrowserWritableField {
                handle: "input_1".to_string(),
                kind: "text_input".to_string(),
                label: "Reply".to_string(),
                placeholder: None,
                value_preview: "".to_string(),
                value_character_count: 0,
                max_length: None,
                required: false,
                disabled: false,
                captured_at: chrono::Utc::now().to_rfc3339(),
            }],
            adapter_health: BrowserAdapterHealth::default(),
            character_count: 30,
            truncated: false,
        }
    }
}
