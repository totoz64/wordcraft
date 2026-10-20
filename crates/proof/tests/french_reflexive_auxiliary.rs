//! Explicit real-data gate for the bounded reflexive auxiliary rule.
#![cfg(not(target_arch = "wasm32"))]
use wordcraft_proof::{IssueKind, Options, ProofLang, check_text, check_text_with, fr};

const MESSAGE: &str = "Auxiliaire être avec un verbe pronominal";

#[test]
#[ignore = "requires the real French morphology in WORDCRAFT_DICTIONARIES"]
fn real_reflexive_auxiliary_examples_preserve_participles_and_clitics() {
    let folder = std::path::PathBuf::from(std::env::var_os("WORDCRAFT_DICTIONARIES").expect("explicit real dictionary folder"));
    let actual = fr::morphology::installed_path().expect("real morphology must be installed");
    assert_eq!(actual.parent(), Some(folder.as_path()), "do not silently use another installation");
    match fr::morphology::data_status() {
        fr::morphology::DataStatus::Ready { analyses, .. } => assert_eq!(analyses, 274_427),
        status => panic!("real finite morphology missing: {status:?}"),
    }
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, fixed) in [
            ("Je m'ai trompée.", "Je me suis trompée."),
            ("Tu t'as expliqué.", "Tu t’es expliqué."),
            ("Il s'a demandé pourquoi.", "Il s’est demandé pourquoi."),
            ("Elle s'a préparée pour le départ.", "Elle s’est préparée pour le départ."),
            ("On se a offert une pause.", "On s’est offert une pause."),
            ("Nous nous avons parlé.", "Nous nous sommes parlé."),
            ("Vous vous avez écrit.", "Vous vous êtes écrit."),
            ("Ils se ont aidés.", "Ils se sont aidés."),
            ("Elles se ont aperçues du problème.", "Elles se sont aperçues du problème."),
            ("Je ne m'ai pas trompée.", "Je ne me suis pas trompée."),
            ("Tu ne t'avais jamais trompé.", "Tu ne t’étais jamais trompé."),
            ("Il ne s'a plus souvenu du lieu.", "Il ne s’est plus souvenu du lieu."),
            ("Nous ne nous avons pas parlé.", "Nous ne nous sommes pas parlé."),
            ("Vous vous avez déjà offert une pause.", "Vous vous êtes déjà offert une pause."),
            ("Elles se ont bien préparées.", "Elles se sont bien préparées."),
            ("Je m'en ai souvenu.", "Je m’en suis souvenu."),
            ("Il s'y a inscrit.", "Il s’y est inscrit."),
            ("Je me les ai procurés.", "Je me les suis procurés."),
            ("Elle se les a offertes.", "Elle se les est offertes."),
            ("Il s'en avait aperçu.", "Il s’en était aperçu."),
            ("Je m'aurai levée avant huit heures.", "Je me serai levée avant huit heures."),
            ("Tu t'aurais couché plus tôt.", "Tu te serais couché plus tôt."),
            ("Elle s'avait demandé pourquoi.", "Elle s’était demandé pourquoi."),
            ("Ils s'avaient enfuis.", "Ils s’étaient enfuis."),
            ("Nous nous aurions sentis mieux.", "Nous nous serions sentis mieux."),
            ("Je m'ai rendu compte de l'oubli.", "Je me suis rendu compte de l'oubli."),
            ("Elle s'a rendu compte de l'oubli.", "Elle s’est rendu compte de l'oubli."),
            ("Vous vous avez fait entendre.", "Vous vous êtes fait entendre."),
            ("— Je m'ai trompée.", "— Je me suis trompée."),
            ("Je ne m'y ai pas trompée.", "Je ne m’y suis pas trompée."),
            ("Tu ne t'en avais jamais souvenu.", "Tu ne t’en étais jamais souvenu."),
            ("Nous nous y avions inscrits.", "Nous nous y étions inscrits."),
            ("Vous vous en auriez souvenus.", "Vous vous en seriez souvenus."),
            ("Il se l'a préparé.", "Il se l’est préparé."),
            ("Tu te la avais offerte.", "Tu te l’étais offerte."),
            ("Nous nous l'avions offert.", "Nous nous l’étions offert."),
        ] {
            let issues = fr::grammar::check_in(text, lang);
            assert_eq!(issues.len(), 1, "{lang:?}: {text}: {issues:?}");
            let issue = &issues[0];
            assert_eq!(issue.message, MESSAGE, "{text}");
            assert_eq!(issue.lang, lang);
            assert_eq!(issue.suggestions.len(), 1, "{text}");
            let actual = format!("{}{}{}", &text[..issue.start], issue.suggestions[0], &text[issue.end..]);
            assert_eq!(actual, fixed, "{text}");
            assert!(fr::grammar::check_in(&actual, lang).is_empty(), "fixed text: {actual}");
        }
        for text in [
            "Tu m’as confié un secret.",
            "Elle lui a offert une place.",
            "Il nous a préparé un repas.",
            "Nous vous avons écrit.",
            "Vous nous avez parlé.",
            "Ils se sont parlé.",
            "Elles se sont écrit.",
            "Je me suis procuré une place.",
            "Elle s’est rendu compte du problème.",
            "Je m’étais trompée.",
            "Je l’ai écrit.",
            "Tu m’avais préparé un repas.",
            "Nous vous aurions offert une place.",
            "Elle se l’est préparé.",
            "Je ne m’y suis pas trompée.",
            "Nous nous l’étions offert.",
        ] {
            assert!(fr::grammar::check_in(text, lang).is_empty(), "correct: {text}");
        }
        // These are unsupported errors, not evidence of correct-text precision.
        for text in [
            "Il s'a mort.",
            "Il s'a plu des pierres.",
            "Elle s'a agi du problème.",
            "Je m'ai été prêt.",
            "Je m'ai eu un cadeau.",
            "Nous se avons trompés.",
            "Je me l'ai procurée.",
            "« Je m'ai trompée. »",
        ] {
            assert!(fr::grammar::check_in(text, lang).iter().all(|issue| issue.message != MESSAGE), "unsupported: {text}");
        }
    }
    real_auxiliary_context_respects_languages_options_and_spelling_priority();
}

fn real_auxiliary_context_respects_languages_options_and_spelling_priority() {
    assert!(matches!(fr::morphology::data_status(), fr::morphology::DataStatus::Ready { analyses: 274_427, .. }));
    // The real Hunspell dictionary accepts m'ai; use an explicit tiny spelling
    // fixture to exercise conflicting marks. This helper is deliberately last
    // in the single test: dictionary installation is process-global. The gate
    // needs only real morphology, independently of installed spelling files.
    fr::install_dictionary("SET UTF-8\n", "2\nJe\net\n").unwrap();
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        let text = "Ils se ont trompés.";
        let full = check_text(text, &[(0..text.len(), lang)]);
        assert!(full.iter().any(|issue| issue.message == MESSAGE));
        assert!(!full.iter().any(|issue| issue.message.starts_with("Élision")), "accepted auxiliary keeps precedence: {full:?}");
        let split = text.find("trompés").unwrap();
        for foreign in [ProofLang::Other, ProofLang::En, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
            let mixed = check_text(text, &[(0..split, lang), (split..text.len(), foreign)]);
            assert!(!mixed.iter().any(|issue| issue.message == MESSAGE), "foreign participle cannot select an auxiliary: {mixed:?}");
            let elision = mixed.iter().find(|issue| issue.message.starts_with("Élision")).expect("the independent French elision remains");
            assert_eq!(elision.lang, lang);
            assert_eq!(&text[elision.start..elision.end], "se ont");
            assert_eq!(elision.suggestions, ["s’ont"]);
            // A rejected auxiliary cannot consume spelling marks on its prefix.
            assert!(mixed.iter().any(|issue| issue.kind == IssueKind::Spelling && &text[issue.start..issue.end] == "ont"));
        }
        let text = "Je m'ai trompée et xyzxyz.";
        let spans = [(0..text.len(), lang)];
        let spelling = check_text_with(text, &spans, &Options::default(), false, false);
        assert!(spelling.iter().all(|issue| issue.kind == IssueKind::Spelling));
        for token in ["m'ai", "trompée", "xyzxyz"] {
            assert!(
                spelling.iter().any(|issue| &text[issue.start..issue.end] == token && issue.kind == IssueKind::Spelling),
                "fixture spelling: {token}: {spelling:?}"
            );
        }
        let full = check_text(text, &spans);
        let auxiliary = full.iter().find(|issue| issue.message == MESSAGE).expect("real auxiliary suggestion");
        assert_eq!(&text[auxiliary.start..auxiliary.end], "m'ai");
        assert!(!full.iter().any(|issue| issue.kind == IssueKind::Spelling && issue.start >= auxiliary.start && issue.end <= auxiliary.end));
        for token in ["trompée", "xyzxyz"] {
            assert!(
                full.iter().any(|issue| issue.kind == IssueKind::Spelling && &text[issue.start..issue.end] == token),
                "context is not a replacement range: {token}: {full:?}"
            );
        }

        let text = "Je ne m'ai pas déjà trompée.";
        for foreign in [ProofLang::Other, ProofLang::En] {
            assert!(!check_text(text, &[(0..text.len(), foreign)]).iter().any(|issue| issue.message == MESSAGE));
        }
        for foreign in [ProofLang::Other, ProofLang::En, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
            // Each decisive part is independently outside this French region.
            for token in ["Je", "ne", "m'ai", "pas", "déjà", "trompée"] {
                let start = text.find(token).unwrap();
                let end = start + token.len();
                let spans = [(0..start, lang), (start..end, foreign), (end..text.len(), lang)];
                assert!(!check_text(text, &spans).iter().any(|issue| issue.message == MESSAGE), "{lang:?}/{foreign:?}: {token}");
            }
        }
        // Adjacent runs of exactly the same language still form one valid context.
        let cut = text.find("déjà").unwrap();
        assert!(check_text(text, &[(0..cut, lang), (cut..text.len(), lang)]).iter().any(|issue| issue.message == MESSAGE));
    }
}
