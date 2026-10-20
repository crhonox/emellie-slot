use omega_cheevos::config::remember_token;
use omega_cheevos::{Credentials, Secret, CHEEVOS_FILE};

fn root_with(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join(CHEEVOS_FILE), text).unwrap();
    dir
}

#[test]
fn reads_a_username_and_password() {
    let c = Credentials::parse("username = Player\npassword = hunter2\n").unwrap();
    assert_eq!(c.username, "Player");
    assert_eq!(c.secret, Secret::Password("hunter2".into()));
    assert_eq!(c.host, None);
}

#[test]
fn reads_a_remembered_token() {
    let c = Credentials::parse("username=Player\ntoken=abc\n").unwrap();
    assert_eq!(c.secret, Secret::Token("abc".into()));
}

#[test]
fn a_freshly_typed_password_beats_an_old_token() {
    let c = Credentials::parse("username=Player\ntoken=stale\npassword=new\n").unwrap();
    assert_eq!(c.secret, Secret::Password("new".into()));
}

#[test]
fn needs_both_a_username_and_a_secret() {
    assert_eq!(Credentials::parse("password=hunter2\n"), None);
    assert_eq!(Credentials::parse("username=Player\n"), None);
    assert_eq!(Credentials::parse("username=\npassword=x\n"), None);
    assert_eq!(Credentials::parse("username=Player\npassword=\n"), None);
}

#[test]
fn an_optional_host_points_at_another_server() {
    let c = Credentials::parse("username=P\ntoken=t\nhost=http://localhost:8080\n").unwrap();
    assert_eq!(c.host.as_deref(), Some("http://localhost:8080"));
}

#[test]
fn a_missing_file_means_no_credentials() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(Credentials::read(dir.path()), None);
}

#[test]
fn remembering_the_token_drops_the_password_and_keeps_everything_else() {
    let dir = root_with("# my account\nusername = Player\npassword = hunter2\nhost = x\n");
    remember_token(dir.path(), "abc").unwrap();
    let text = std::fs::read_to_string(dir.path().join(CHEEVOS_FILE)).unwrap();
    assert_eq!(
        text,
        "# my account\nusername = Player\nhost = x\ntoken = abc\n"
    );
    assert_eq!(
        Credentials::read(dir.path()).unwrap().secret,
        Secret::Token("abc".into())
    );
    remember_token(dir.path(), "abc").unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join(CHEEVOS_FILE)).unwrap(),
        text
    );
    let leftovers: Vec<_> = std::fs::read_dir(dir.path().join("Config"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(leftovers, ["retroachievements.txt"]);
}

#[test]
fn settings_save_an_account_and_sign_out_of_it() {
    use omega_cheevos::config::{account, sign_in_with, sign_out, Account};
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        account(dir.path()),
        Account {
            enabled: true,
            ..Account::default()
        }
    );
    sign_in_with(dir.path(), "Player", "hunter2").unwrap();
    let c = Credentials::read(dir.path()).unwrap();
    assert_eq!(
        (c.username.as_str(), c.secret),
        ("Player", Secret::Password("hunter2".into()))
    );
    remember_token(dir.path(), "tok").unwrap();
    assert!(account(dir.path()).signed);
    sign_in_with(dir.path(), "Other", "pw").unwrap();
    assert_eq!(
        Credentials::read(dir.path()).unwrap().secret,
        Secret::Password("pw".into())
    );
    sign_out(dir.path()).unwrap();
    let a = account(dir.path());
    assert_eq!((a.username.as_str(), a.signed), ("Other", false));
    assert_eq!(Credentials::read(dir.path()), None);
}

#[test]
fn turned_off_in_settings_there_is_nothing_to_sign_in_with() {
    use omega_cheevos::config::{account, set_enabled};
    let dir = root_with("username=Player\ntoken=abc\n");
    set_enabled(dir.path(), false).unwrap();
    assert_eq!(Credentials::read(dir.path()), None);
    assert!(!account(dir.path()).enabled);
    set_enabled(dir.path(), true).unwrap();
    assert!(Credentials::read(dir.path()).is_some());
}
