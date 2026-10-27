use omega_zero::link_kind::{armgba_port, both_ports, link_kind, LinkKind};

#[test]
fn the_wireless_adapter_games_are_wireless() {
    for code in ["BMGE", "BTME", "BR5E", "BRBE", "BDGE", "B4UE", "B85A"] {
        assert_eq!(link_kind(code, "", true), LinkKind::Wireless, "{code}");
    }
    for code in ["BPEE", "BPRE", "BPGE"] {
        assert_eq!(link_kind(code, "", true), LinkKind::Cable, "{code}");
    }
    for (code, title) in [
        ("BPRE", "POKEMON FIRE"),
        ("BPGE", "POKEMON LEAF"),
        ("BPEE", "POKEMON EMER"),
    ] {
        assert_eq!(link_kind(code, title, true), LinkKind::Wireless, "{code}");
    }
}

#[test]
fn ruby_sapphire_and_advance_wars_use_the_cable() {
    for code in ["AXVE", "AXPE", "AWRE", "AW2E"] {
        assert_eq!(
            link_kind(code, "POKEMON RUBY", true),
            LinkKind::Cable,
            "{code}"
        );
    }
}

#[test]
fn a_pokemon_hack_links_by_cable() {
    for (code, title, clean) in [
        ("BPEE", "POKEMON EMER", false),
        ("BPRE", "PKMN RADICAL", true),
        ("ZZZZ", "POKEMON EMER", true),
        ("", "POKEMON", true),
    ] {
        assert_eq!(
            link_kind(code, title, clean),
            LinkKind::Cable,
            "{code} {title} {clean}"
        );
    }
}

#[test]
fn everything_else_is_the_cable() {
    assert_eq!(link_kind("OMTE", "OMEGA TEST", true), LinkKind::Cable);
    assert_eq!(link_kind("", "", true), LinkKind::Cable);
}

#[test]
fn armgba_is_put_on_the_port_chosen() {
    assert_eq!(armgba_port(LinkKind::Cable), "cable");
    assert_eq!(armgba_port(LinkKind::Wireless), "wireless");
}

#[test]
fn only_pokemon_links_both_ways() {
    for (code, title) in [
        ("BPEE", "POKEMON EMER"),
        ("BPRE", "POKEMON FIRE"),
        ("AXVE", "POKEMON RUBY"),
        ("BPEE", "PKMN HACK"),
        ("ZZZZ", "POKEMON"),
    ] {
        assert!(both_ports(code, title), "{code} {title}");
    }
    for (code, title) in [
        ("BMGE", "MARIOGOLFADV"),
        ("AWRE", "ADVANCEWARS"),
        ("2ATE", "APOTRIS"),
        ("", ""),
    ] {
        assert!(!both_ports(code, title), "{code} {title}");
    }
}
