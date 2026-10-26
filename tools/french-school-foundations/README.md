# Analyseur lexical scolaire — première étape

Statut : laboratoire original, séparé du correcteur WordCraft. Les tests doivent être exécutés avant de prétendre l'outil validé. Aucun diagnostic ni remplacement de texte.

Le programme reçoit une phrase et renvoie pour chaque segment son texte exact, ses plages UTF-8/UTF-16 et les lectures lexicales connues : lemme, nature et traits. Il conserve les lectures multiples ; il ne choisit pas selon le contexte.

## Essayer

Depuis ce dossier :

    cargo run --release -- --text "Les petits chiens jouent."
    cargo run --release -- --text "porte suis les chevaux"
    cargo run --release -- --stdin

La sortie est du JSON. Le texte fourni est français ; aucune détection de langue ni prise en compte des langues de plages d'un document n'est implémentée. Ce programme n'est pas encore raccordé à WordCraft.

## Les neuf classes définies

| Nature | Première définition de travail |
|---|---|
| Nom | Désigne notamment un être, un objet ou une idée ; porte un genre et une forme en nombre. |
| Déterminant | Introduit le nom dans un groupe nominal ; ses traits varient selon le type. |
| Adjectif | Apporte une caractérisation ; peut varier en genre et en nombre. |
| Pronom | Peut occuper des places grammaticales sans exprimer un nom ; il ne remplace pas nécessairement un nom déjà écrit. |
| Verbe | Possède des formes conjuguées et non personnelles ; une terminaison isolée ne suffit pas à le reconnaître. |
| Adverbe | Mot généralement invariable qui peut modifier d'autres éléments ou l'énoncé. |
| Préposition | Introduit un groupe dépendant d'un autre élément. |
| Conjonction | Relie des éléments ou introduit une proposition, selon son type. |
| Interjection | Exprime notamment une réaction ; son emploi n'exige pas une phrase sujet–verbe. |

Ces définitions courtes guident les données ; elles ne sont pas des règles permettant de classifier tout mot inconnu. La ponctuation, les nombres écrits en chiffres et les symboles sont des types de segments distincts, pas de nouvelles natures de mots.

## Ce qui est effectivement reconnu

Lexique miniature original : noms et pluriels explicitement listés, quelques adjectifs et leurs formes, mots grammaticaux et huit verbes (jouer, porter, parler, manger, aboyer, être, avoir, suivre). Pour ces verbes : infinitif, présent de l'indicatif, du subjonctif et de l'impératif seulement. Pas de temps composés ou d'autres temps, pas de génération de lemmes inconnus par suffixe.

Les tableaux de formes ont été écrits pour ce laboratoire à partir des connaissances grammaticales usuelles ; aucun code de moteur tiers ni table du prototype #27 n'est repris. Chaque lecture porte provenance=tables_originales_scolaires_v1. Ce n'est ni un dictionnaire exhaustif ni une validation indépendante des tableaux.

Les/pluriel ne reçoit pas un genre masculin arbitraire. Heureux conserve deux nombres. Suis conserve être et suivre, avec la lecture impérative de suivre. Porte garde nom et verbe. Les formes verbales restent reconnues même dans Ils joue ; aucune correction d'accord n'est produite.

## Références et limites

Progression guidée par la présentation et le sommaire publics de Bled 6e–5e, Hachette, édition 2024 : https://www.enseignants.hachette-education.com/livre/bled-6e-5e-livre-eleve-ed-2024-9782017260301/ . Le manuel complet n'a pas été consulté et ses leçons/exercices ne sont pas reproduits.

Terminologie : ministère de l'Éducation nationale, https://eduscol.education.gouv.fr/media/1872/download . Les noms des classes viennent de la grammaire scolaire ; les définitions, exemples et code ici sont originaux. Pas d'affiliation ou certification Bled/Hachette.

Tous les résultats positifs sont lectures_connues_non_exhaustives. Un mot absent est inconnu_du_lexique : il n'est pas déclaré fautif. La reconnaissance utilise une copie NFC en minuscules, sans transformer la source. Les préfixes élidés l'/l’, j'/j’ et d'/d’ sont séparés du mot suivant ; les autres apostrophes internes restent dans un mot. Les mots composés à trait d'union sont segmentés et leur analyse d'ensemble est différée. Les symboles complexes peuvent produire plusieurs segments Unicode ; ce n'est pas un découpage en graphèmes.

Le mode lexical seul ne calcule pas de fonctions ou groupes. Le mode --syntax, décrit ci-dessous, ajoute des hypothèses structurelles limitées. Aucun diagnostic, classement des corrections, interprétation du sens ou apprentissage automatique. Maximum 65536 octets et 8192 segments ; les dépassements sont des erreurs explicites.

## Vérifier

    cargo test
    cargo clippy --all-targets -- -D warnings
    cargo fmt -- --check

Les 16 cas manuels proviennent d'une relecture distincte du code. Le fichier data/independent-cases.json adapte leurs clés (gender→genre, number→nombre, person→personne, tense→temps, subtype→type) et les libellés sans accents du format Rust. Une contraction de le est représentée par un article contracté portant explicitement sa préposition, son lemme et ses traits ; elle ne crée pas de fausses positions dans la source. Les minima ne sont pas un score global ou aveugle ; ils servent à vérifier le contrat exposé.

La validation n'est pas celle des 59048 anciens cas ni celle du correcteur officiel. Le suivi automatique reste désactivé.


## Deuxième étape : groupes et relations locales, sur demande

    cargo run --locked --release -- --syntax --text "Le chien des voisins aboie."
    cargo run --locked --release -- --syntax --text "Je suis le voisin."

Le résultat contient analyse_lexicale (inchangée) et analyse_syntaxique. Le mode lexical historique conserve son format. Il ne faut pas confondre lectures lexicales disponibles et lectures qui peuvent jouer un rôle dans une hypothèse syntaxique.

Portée : une phrase déclarative simple, sujet antéposé pronom personnel ou groupe nominal déterminant + adjectifs antéposés + nom. Des compléments du nom introduits par de/des/du/d’ sont imbriqués jusqu’à deux niveaux. Les liens sujet, épithète et complément_du_nom pointent vers les tokens originaux. La tête du sujet reste chien dans Le chien des voisins aboie. Aucune vérification d’accord ne décide si une structure est reconnue : Ils joue reste analysable.

Le verbe du cadre déclaratif garde ses lectures indicatives/subjonctives/conditionnelles connues, sans filtrage de personne ou de nombre. Une lecture impérative reste dans les tokens lexicaux mais n’est pas attachée au sujet exprimé de ce cadre. Une lecture structurale n’est pas une preuve de correction grammaticale ou d’intention.

Pour les formes pronominales je/j’/tu/il/ils/elle/elles/on, un GN final entièrement reconnu peut être gardé comme reste_non_analyse_tokens, avec un statut explicitement partiel. Il ne reçoit aucune fonction. Je suis le voisin conserve être/suivre et ne choisit ni attribut ni objet. Le même cas avec nous/vous est refusé pour ne pas décider un sujet dans une possible inversion. Si le reste contient une forme à lecture verbale, l’analyse s’abstient : Tu portes la porte est donc hors portée à cette étape.

Négation, relatives, coordination, clitiques objets, sujets inversés, attributs/objets analysés et temps composés ne sont pas implémentés. L’inconnu, les ponctuations internes et les cas trop grands font abstention. Les bornes sont 128 tokens, 64 lectures par token, 16 hypothèses et 4096 unités de recherche ; aucune liste tronquée d’hypothèses n’est publiée en cas d’épuisement.

Douze cas originaux supplémentaires, rédigés par une personne de la revue distincte du code, sont conservés dans data/syntax-cases.json. Le test de contrat vérifie les relations sujet–verbe, leurs têtes/plages, le rattachement des compléments et les abstentions. Les traits de personne/nombre/genre figurant dans les annotations manuelles restent des informations de référence : leur propagation calculée au groupe sujet n’est pas encore implémentée et n’est pas revendiquée par ce test.

Historique : première étape lexicale validée sur 5a1bea6 ; deuxième étape compilée et testée localement avec Rust1.98.1GNU dédié au projet. Les résultats hébergés de la première étape ne valident pas les nouveaux fichiers ; voir les preuves exactes du nouvel incrément.
