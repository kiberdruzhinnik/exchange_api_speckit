use std::{
    io::{BufRead, BufReader, Read},
    net::TcpListener,
    process::{Command, Stdio},
};

fn capture_logs(color_setting: Option<&str>) -> String {
    let temp_dir = tempfile::tempdir().expect("create temporary directory");
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve an ephemeral port");
    let listen_addr = listener.local_addr().expect("read ephemeral address");
    drop(listener);

    let mut command = Command::new(env!("CARGO_BIN_EXE_exchange-api"));
    command
        .env("EXCHANGE_API_LISTEN_ADDR", listen_addr.to_string())
        .env(
            "EXCHANGE_API_HISTORY_CACHE_DB_PATH",
            temp_dir.path().join("history.sqlite3"),
        )
        .env_remove("NO_COLOR")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    if let Some(value) = color_setting {
        command.env("EXCHANGE_API_LOG_COLOR", value);
    } else {
        command.env_remove("EXCHANGE_API_LOG_COLOR");
    }

    let mut child = command.spawn().expect("start exchange-api binary");
    let mut output = String::new();
    let stdout = child.stdout.take().expect("capture standard output");
    let mut stdout = BufReader::new(stdout);
    stdout
        .read_line(&mut output)
        .expect("read startup log line");

    let signal_result = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("send SIGINT to exchange-api");
    assert!(signal_result.success(), "failed to send SIGINT to service");

    stdout
        .read_to_string(&mut output)
        .expect("capture shutdown log lines");
    let exit_status = child.wait().expect("wait for exchange-api shutdown");
    assert!(exit_status.success(), "service exited unsuccessfully");
    output
}

fn assert_representative_log_sequence(output: &str) {
    let plain_output = strip_ansi(output);
    let expected_messages = [
        "starting exchange API",
        "received Ctrl+C (SIGINT)",
        "shutdown signal received; draining in-flight requests",
    ];

    let mut previous_position = None;
    for message in expected_messages {
        let position = plain_output
            .find(message)
            .unwrap_or_else(|| panic!("missing {message:?} in logs: {plain_output:?}"));
        if let Some(previous_position) = previous_position {
            assert!(
                previous_position < position,
                "log message {message:?} appeared out of order: {plain_output:?}"
            );
        }
        let line_start = plain_output[..position]
            .rfind('\n')
            .map_or(0, |newline| newline + 1);
        let line_prefix = &plain_output[line_start..position];
        assert!(
            line_prefix.contains("INFO"),
            "message {message:?} lost its INFO severity: {plain_output:?}"
        );
        previous_position = Some(position);
    }
}

fn strip_ansi(output: &str) -> String {
    let mut plain = String::with_capacity(output.len());
    let mut chars = output.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for code in chars.by_ref() {
                if ('@'..='~').contains(&code) {
                    break;
                }
            }
        } else {
            plain.push(character);
        }
    }
    plain
}

#[test]
fn disabling_log_colors_removes_ansi_and_preserves_message_and_severity() {
    let output = capture_logs(Some("off"));
    let enabled_output = capture_logs(Some("true"));

    assert_representative_log_sequence(&output);
    assert_representative_log_sequence(&enabled_output);
    assert!(
        !output.contains("\x1b["),
        "unexpected ANSI sequence: {output:?}"
    );
    assert!(
        enabled_output.contains("\x1b["),
        "explicitly enabled output should contain ANSI sequences: {enabled_output:?}"
    );
}

#[test]
fn log_colors_are_enabled_by_default_and_for_other_values() {
    for (setting, label) in [
        (None, "unset"),
        (Some("true"), "explicitly enabled"),
        (Some("unexpected"), "unrecognized value"),
    ] {
        let output = capture_logs(setting);
        assert!(
            output.contains("starting exchange API"),
            "{label}: {output:?}"
        );
        assert!(output.contains("INFO"), "{label}: {output:?}");
        assert!(
            output.contains("\x1b["),
            "{label} should retain ANSI sequences: {output:?}"
        );
    }
}
