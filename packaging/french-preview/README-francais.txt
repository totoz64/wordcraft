WORDCRAFT — PRÉVERSION FRANÇAISE POUR ESSAI
Cette version expérimentale rassemble les changements de la branche indiquée dans build-manifest.json.
Elle n'est pas une publication officielle du projet. Aucune PR de contribution n'est fusionnée par sa fabrication.

1. Extrayez TOUT le ZIP dans un dossier où vous pouvez écrire.
2. Double-cliquez Lancer-WordCraft.cmd.
3. Au premier lancement, le lanceur télécharge le dictionnaire français public épinglé (environ 1,5 Mo) dans dictionaries et vérifie ses deux empreintes SHA-256. Le réseau est nécessaire une fois ; aucun dictionnaire n'est contenu dans l'archive.
4. Le document artificiel essai-francais.docx s'ouvre. Appuyez sur F7, cliquez une proposition, puis Ctrl+Z pour annuler.

Le lanceur utilise un profil séparé dans profile : il conserve vos réglages et les mots ajoutés sans réutiliser le profil de votre installation normale. Ne lancez pas directement wordcraft.exe si vous souhaitez cette isolation.
Le dictionnaire provient de LibreOffice/dictionaries, révision 67032ee957631908098a72edf48dc81cac27e369, Dicollecte 7.7. Sa notice MPL 2.0 est téléchargée avec les fichiers. Les adresses et empreintes sont dans dictionary-source.json.
La police et les catalogues disponibles sont embarqués pendant la compilation ; certaines nouvelles étiquettes peuvent encore apparaître en anglais.
Le logiciel cherche actuellement les mises à jour officielles au démarrage. Pour conserver le SHA de cette préversion, n'appliquez pas de mise à jour pendant l'essai.

ESSAI COURT
- maisonn : maison doit être proposé ; un clic remplace, Ctrl+Z restaure.
- CCe : Ce doit être proposé en gardant la majuscule.
- Il à faim. : a doit être proposé.
- Elle a préparer le repas. : préparé doit être proposé.
- Cette boîte ne contient pas grand chose. : grand-chose doit être proposé.
- Les mots centimètres, grammes et watts sont des témoins corrects.
Ces contrôles sont bornés : ce ne sont ni un score de qualité général, ni une promesse de correction de toutes les fautes.

acceptance.json décrit les contrôles attendus ; acceptance-result.json contient le résultat CLI réellement mesuré sur cette compilation. Il reste indispensable de vérifier visuellement la fenêtre, les cartes de suggestions et le clic natif. Les captures clair/sombre du même SHA sont publiées comme artefact distinct : une phrase française de cette démonstration et l’ancien exemple synthétique anglais sont conservés séparément.
L'archive contient une application Windows x64 et un outil CLI. Le CRT est statique : aucun redistribuable Visual C++ séparé n'est nécessaire. Le système et son pilote graphique doivent permettre à l'application de s'ouvrir.
Ce build de préversion est non signé. S'il est bloqué par Windows, conservez le message exact ; ce guide ne demande pas de désactiver une protection.
En cas d'échec de téléchargement, vérifiez la connexion ; le lanceur s'arrête avant d'ouvrir un correcteur sans dictionnaire. Les fichiers partiels portent le suffixe .download. Ne remplacez pas les empreintes pour ignorer un échec.
Les journaux de cette préversion sont sous profile/logs. Son SHA et les empreintes des exécutables sont dans build-manifest.json ; l'empreinte du ZIP est dans SHA256SUMS.txt à côté du ZIP.
