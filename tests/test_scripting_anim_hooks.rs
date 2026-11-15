use mossambi::core::scripting::{execute_script_sync, ScriptLang};

#[test]
fn test_rhai_animation_hook_execution() {
    let script = r#"
        let effect_ok = mossambi_anim_set_window_effect("genie");
        let spring_ok = mossambi_anim_tune_spring("sidebar", 280.0, 0.95, 1.0);
        let toggle_ok = mossambi_sidebar_toggle();
        mossambi_log("rhai animation configured");
        effect_ok && spring_ok && toggle_ok
    "#;
    let res = execute_script_sync(script, ScriptLang::Rhai, None, &[]);
    assert!(
        res.success,
        "Rhai script should execute successfully: {}",
        res.error.unwrap_or_default()
    );
}

#[test]
fn test_lua_animation_hook_execution() {
    let script = r#"
        mossambi.anim.set_window_effect("scale")
        mossambi.anim.tune_spring("sidebar", 260.0, 0.90, 1.0)
        mossambi.sidebar.toggle()
        mossambi_log("lua animation configured")
        return true
    "#;
    let res = execute_script_sync(script, ScriptLang::Lua, None, &[]);
    assert!(
        res.success,
        "Lua script should execute successfully: {}",
        res.error.unwrap_or_default()
    );
}
