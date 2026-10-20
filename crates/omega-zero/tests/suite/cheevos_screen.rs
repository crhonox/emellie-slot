use omega_cheevos::config::{account, Account};
use omega_cheevos::{Credentials, Secret};
use omega_input::{Action, Btn};
use omega_zero::cheevos::Status;
use omega_zero::cheevos_screen::{status_lines, CheevosScreen, Outcome, Sel, Step};

fn press(s: &mut CheevosScreen, a: &Account, root: &std::path::Path, btn: Btn) -> Outcome {
    s.input(a, Action::GbaDown(btn), Some(root))
}

/// Types `text` on the keyboard as a player would not, straight into its line, then STARTs.
fn type_in(s: &mut CheevosScreen, a: &Account, root: &std::path::Path, text: &str) -> Outcome {
    s.typing.as_mut().unwrap().keys.text = text.to_string();
    press(s, a, root, Btn::Start)
}

#[test]
fn an_account_typed_in_is_saved_to_sign_in_with() {
    let d = tempfile::tempdir().unwrap();
    let mut s = CheevosScreen::default();
    let a = account(d.path());
    assert_eq!(press(&mut s, &a, d.path(), Btn::Down), Outcome::Stay);
    assert_eq!(s.sel, Sel::Account);
    press(&mut s, &a, d.path(), Btn::A);
    assert_eq!(s.typing.as_ref().unwrap().step, Step::User);
    assert_eq!(type_in(&mut s, &a, d.path(), "Player"), Outcome::Stay);
    let t = s.typing.as_ref().unwrap();
    assert_eq!((t.step, t.user.as_str()), (Step::Password, "Player"));
    assert!(t.prompt().contains("Player"));
    assert_eq!(type_in(&mut s, &a, d.path(), "hunter2"), Outcome::Changed);
    assert!(s.typing.is_none());
    let c = Credentials::read(d.path()).unwrap();
    assert_eq!(c.username, "Player");
    assert_eq!(c.secret, Secret::Password("hunter2".into()));
}

#[test]
fn the_switch_turns_retroachievements_off_and_on() {
    let d = tempfile::tempdir().unwrap();
    let mut s = CheevosScreen::default();
    assert_eq!(
        press(&mut s, &account(d.path()), d.path(), Btn::Right),
        Outcome::Changed
    );
    assert!(!account(d.path()).enabled);
    assert_eq!(
        press(&mut s, &account(d.path()), d.path(), Btn::A),
        Outcome::Changed
    );
    assert!(account(d.path()).enabled);
}

#[test]
fn y_signs_out_but_keeps_the_name() {
    let d = tempfile::tempdir().unwrap();
    omega_cheevos::config::sign_in_with(d.path(), "Player", "pw").unwrap();
    let mut s = CheevosScreen {
        sel: Sel::Account,
        typing: None,
    };
    assert_eq!(
        press(&mut s, &account(d.path()), d.path(), Btn::Y),
        Outcome::Changed
    );
    let a = account(d.path());
    assert_eq!((a.username.as_str(), a.signed), ("Player", false));
    // with nothing to sign out of, Y does nothing
    assert_eq!(press(&mut s, &a, d.path(), Btn::Y), Outcome::Stay);
}

#[test]
fn b_backs_out_of_the_keyboard_then_the_screen() {
    let d = tempfile::tempdir().unwrap();
    let a = account(d.path());
    let mut s = CheevosScreen {
        sel: Sel::Account,
        typing: None,
    };
    press(&mut s, &a, d.path(), Btn::A);
    assert_eq!(press(&mut s, &a, d.path(), Btn::B), Outcome::Stay);
    assert!(s.typing.is_none());
    assert_eq!(press(&mut s, &a, d.path(), Btn::B), Outcome::Close);
}

#[test]
fn the_notes_say_where_the_account_stands() {
    let signed = Account {
        username: "Player".into(),
        signed: true,
        enabled: true,
    };
    let first = |a: &Account, s: &Status, online| status_lines(a, s, online)[0].clone();
    let off = Account {
        enabled: false,
        ..signed.clone()
    };
    assert!(first(&off, &Status::Off, true).contains("off"));
    assert!(first(&Account::default(), &Status::NoAccount, true).contains("Sign in"));
    assert!(first(&signed, &Status::Waiting, false).contains("Wi-Fi"));
    assert!(first(&signed, &Status::SigningIn, true).contains("Signing in as Player"));
    assert_eq!(
        first(&signed, &Status::SignedIn("Player".into()), true),
        "Signed in as Player."
    );
    assert!(first(&signed, &Status::Rejected("bad".into()), true).contains("refused"));
    for lines in [
        status_lines(&signed, &Status::SignedIn("Player".into()), true),
        status_lines(&signed, &Status::Rejected("x".into()), true),
        status_lines(&Account::default(), &Status::NoAccount, true),
    ] {
        for line in lines {
            assert!(
                line.chars().count() <= 44,
                "too wide for the panel: {line:?}"
            );
        }
    }
}
