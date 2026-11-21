# Bienvenue
ScreenCapture capture l'écran, annote les images et enregistre des vidéos. Presque chaque action peut se faire de plusieurs façons : raccourci clavier, bouton de l'accueil, menu de la zone de notification ou souris. Cette aide les présente toutes.
![accueil]
- **Accueil** : lancer une capture et retrouver l'historique.
- **Paramètres** : dossier, format, raccourcis, conservation, langue, mises à jour.
- **Aide** : cette page. Touche `F1` depuis l'accueil.
> ScreenCapture reste disponible dans la zone de notification (près de l'horloge) : la croix de la fenêtre la masque sans quitter l'application.
> Sur Mac, `Ctrl` correspond à `⌘` et les raccourcis de capture se choisissent dans les Paramètres.

# Faire une capture
Six types de capture :
- **Zone** : une partie de l'écran, choisie à la souris.
- **Plein écran** : tous les écrans à la fois.
- **Fenêtre** : la fenêtre au premier plan.
- **Dernière zone** : la même zone que la capture précédente, sans la redessiner.
- **Différée** : une zone, après un compte à rebours (pour ouvrir un menu par exemple).
- **Vidéo** : enregistrement d'une zone, voir « Enregistrer une vidéo ».
## Pour lancer une capture
- Le **raccourci clavier**, depuis n'importe quelle application : `Impr. écran` pour une zone, `Ctrl + Impr. écran` pour le plein écran, `Alt + Impr. écran` pour une fenêtre, `Maj + Impr. écran` pour la dernière zone, `Ctrl + Maj + Impr. écran` pour la vidéo. Ils se changent dans les Paramètres.
- Les **cartes de l'accueil** (Zone, Plein écran, Fenêtre…).
- Le **menu de la zone de notification** : clic droit sur l'icône de ScreenCapture.
- Le **bouton de la barre de gauche** (en haut), pour une zone.
- La **ligne de commande** : `ScreenCapture.exe --cmd region` (voir « Pour aller plus loin »).
## Choisir la zone
![selection]
- **Glisser** pour dessiner la zone ; relâcher capture.
- **Cliquer** sur une fenêtre pour la capturer entière (elle est encadrée au survol).
- `Entrée` capture l'écran entier, `Échap` annule.
- `C` copie la couleur sous le curseur (code hexadécimal) ; la loupe montre les pixels agrandis.
- Le bandeau d'aide s'écarte de lui-même s'il gêne la zone que vous sélectionnez.
## Après la capture
Selon le réglage « Après la capture » :
- **Miniature** : une vignette s'affiche quelques secondes. Cliquer dessus ouvre l'éditeur.
- **Éditeur** : l'image s'ouvre directement pour être annotée.
- **Rien** : la capture est seulement enregistrée et/ou copiée.
La capture peut aussi être copiée automatiquement dans le presse-papiers et enregistrée dans le dossier des captures (Paramètres).
## Capture avec défilement
Pour capturer une page plus haute que l'écran (page web, document…) : carte **Défilement** de l'accueil, ou menu de la zone de notification.
- Sélectionner la zone qui défile, de préférence sans la barre de défilement.
- La page défile toute seule vers le bas et les images sont assemblées ; la capture s'arrête en bas de la page, ou avec **Terminer** / `Échap`.
- Les en-têtes et pieds de page fixes ne sont gardés qu'une fois.
> Sur Mac, le défilement automatique demande l'autorisation **Accessibilité** (Réglages Système > Confidentialité et sécurité). Sans elle, faites défiler vous-même la page avec la molette ou le pavé tactile, puis **Terminer**.

# Annoter dans l'éditeur
![editeur]
L'éditeur s'ouvre après une capture, par un double-clic sur une vignette de l'historique, par le bouton « Ouvrir une image », ou en glissant un fichier image dans la fenêtre.
## Les outils
Chaque outil a une touche (sans Ctrl) :
- `V` Sélection, `A` Flèche, `L` Ligne, `R` Rectangle, `E` Ellipse, `P` Main levée.
- `H` Surligneur, `B` Flou / pixellisation, `M` Loupe.
- `T` Texte, `U` Bulle, `N` Numérotation, `S` Tampon.
- `C` Rogner, `X` Découper une bande, `D` Déplacer une zone, `I` Pipette.
Tenir `Maj` en traçant donne des angles de 45° (flèches, lignes) ou des formes régulières (carré, cercle).
## Modifier une forme
- **Sélectionner** : outil Sélection, puis clic sur la forme. Les poignées permettent de la redimensionner.
- **Déplacer** : glisser la forme, ou les flèches du clavier (`Maj` + flèche : 10 pixels).
- **Se déplacer dans l'image zoomée** : maintenir le **clic droit** (ou le bouton du milieu) et déplacer la souris ; la molette fait aussi défiler.
- **Bulles** : quatre types (arrondie, rectangulaire, ovale, pensée) dans le panneau de droite. La **poignée de la pointe** se glisse pour l'orienter ou l'allonger sans déplacer la bulle ; « Pointe par défaut » la remet en place.
- **Dupliquer** : `Ctrl + D`, ou le bouton « Dupliquer » du panneau de droite. Un numéro dupliqué prend le numéro suivant de la série.
- **Modifier le contenu** : **double-clic** sur un texte, une bulle, un numéro ou un tampon (ou bouton « Modifier le contenu »). Pour un numéro, tapez la nouvelle valeur (5, e, IV… selon le style) puis `Entrée`.
- **Couleur, épaisseur, taille du texte, remplissage, ombre, pointillés** : panneau de droite, appliqués à la forme sélectionnée, ou aux prochaines formes si rien n'est sélectionné.
- **Premier plan / arrière-plan** et **supprimer** (`Suppr`) : boutons du panneau de droite.
- **Annuler / rétablir** : `Ctrl + Z` / `Ctrl + Y`.
## Numéros et tampons
- **Numérotation** : chaque clic pose le numéro suivant. Le panneau de droite choisit le style (1, 2, 3 · a, b, c · I, II, III…), la prochaine valeur, et peut tout renuméroter dans l'ordre.
- **Tampons** : APPROUVÉ, CONFIDENTIEL, BROUILLON… ou votre propre texte dans le champ **Tampon personnalisé** (il est mémorisé). Cliquer sur l'image pose le tampon ; un double-clic sur un tampon posé modifie son texte.
## Transformer l'image
- **Rogner** (`C`) : glisser la zone à garder, puis `Entrée`.
- **Découper une bande** (`X`) : retire une bande horizontale ou verticale et recolle les deux morceaux (raccord droit ou en zigzag).
- **Déplacer une zone** (`D`) : sélectionner une partie de l'image et la faire glisser ; l'emplacement d'origine est rempli avec la couleur du fond.
- **Pivoter**, **redimensionner** et **extraire le texte** (reconnaissance de caractères) : boutons de la barre du haut.
## Enregistrer et partager
- `Ctrl + S` enregistre, `Ctrl + Maj + S` enregistre sous un autre nom ou format.
- `Ctrl + C` copie l'image annotée dans le presse-papiers.
- **Effets à l'export** (bordure, ombre portée) : ajoutés à l'image enregistrée ou copiée.
- `Ctrl + molette` zoome, `Ctrl + 0` ajuste à la fenêtre ; le bouton du milieu de la souris déplace la vue.

# L'historique des captures
L'accueil liste les captures du dossier, les plus récentes en premier.
- **Filtres** Tout / Images / Vidéos et **recherche** par nom.
- **Ouvrir** : double-clic (éditeur pour une image, lecteur pour une vidéo).
- **Actions rapides** au survol d'une vignette : éditer, copier, épingler, afficher dans l'Explorateur, supprimer. Clic droit : menu complet.
## Sélectionner plusieurs captures
- **La case ronde** en haut à gauche d'une vignette l'ajoute à la sélection d'un simple clic. Ensuite, un clic sur une autre vignette l'ajoute ou la retire, sans touche à tenir.
- Ou bien : `Ctrl` + clic pour ajouter ou retirer, `Maj` + clic pour une plage, `Ctrl + A` pour tout sélectionner.
- Le **bandeau de sélection** affiche le nombre de captures et permet de tout sélectionner, d'épingler ou de **supprimer toute la sélection** en un clic (ou `Suppr`). `Échap` désélectionne.
- Les captures supprimées vont dans la Corbeille, après confirmation.
## Ajouter des images
- `Ctrl + V` colle l'image du presse-papiers dans le dossier des captures.
- Glisser des fichiers image depuis l'Explorateur dans la fenêtre les ajoute.
- Le bouton « Ouvrir une image » ouvre n'importe quelle image dans l'éditeur.

# Enregistrer une vidéo
- Lancer : raccourci `Ctrl + Maj + Impr. écran`, carte **Vidéo** de l'accueil ou menu de notification, puis choisir la zone comme pour une capture.
- **Son** : la touche `M` (ou un clic sur la pastille) choisit avec ou sans le micro avant de commencer. Pendant l'enregistrement, le bouton micro de la barre flottante coupe ou rétablit le son.
- **Arrêter** : bouton de la barre flottante ou même raccourci. **Annuler** : `Échap`.
- Formats MP4, WebM ou GIF, images par seconde, curseur et micro : dans les Paramètres. L'enregistrement utilise FFmpeg (proposé par l'installeur).

# Conservation des captures
Dans les Paramètres, ScreenCapture peut supprimer automatiquement les anciennes captures :
- une **durée** pour les images et une autre pour les vidéos (7, 30 ou 90 jours, 1 an, durée personnalisée ou jamais) ;
- une **taille maximale** du dossier : au-delà, les plus anciennes partent en premier ;
- vers la **Corbeille** (récupérable) ou **définitivement**.
Les captures **épinglées** (punaise sur la vignette) et celle ouverte dans l'éditeur ne sont jamais supprimées ; les fichiers qui ne viennent pas de ScreenCapture ne sont jamais touchés. « Nettoyer maintenant » montre ce qui sera supprimé avant de le faire.

# Paramètres
![parametres]
- **Capture** : dossier, format PNG ou JPEG, enregistrement et copie automatiques, ce qui se passe après la capture, curseur, délai de la capture différée, loupe.
- **Raccourcis clavier** : saisir un raccourci (par exemple `ctrl+shift+KeyS`) puis « Appliquer » ; laisser vide pour le désactiver.
- **Couleurs de l'éditeur** : modifier, ajouter ou retirer les couleurs proposées.
- **Mises à jour** : recherche automatique et bouton « Rechercher maintenant ».
- **Apparence et démarrage** : langue (français, anglais, espagnol, allemand ou celle du système), thème clair ou sombre, couleur d'accent, fermeture dans la zone de notification, démarrage réduit.
- **Lancer au démarrage de Windows** (ou à l'ouverture de session sur Mac) : s'active ou se désactive à tout moment, même si le choix a été fait à l'installation. L'interrupteur montre l'état réel.

# Mises à jour et version portable
## Mises à jour
Quand une nouvelle version est publiée, un bandeau apparaît en haut de la fenêtre. **Mettre à jour** la télécharge, vérifie son empreinte, l'installe et relance ScreenCapture. **Plus tard** masque le bandeau jusqu'au prochain démarrage. Sur Mac, l'image disque est téléchargée puis ouverte : il suffit de glisser l'application dans Applications.
## Version portable
La version portable (fichier `.zip`) s'utilise sans installation, par exemple depuis une clé USB : décompresser le dossier puis lancer `ScreenCapture.exe`. Les réglages et le journal sont enregistrés dans le dossier `data` à côté du programme. Les mises à jour remplacent le programme au même endroit.

# Pour aller plus loin
## Ligne de commande
Une instance déjà lancée se pilote avec `ScreenCapture.exe --cmd` suivi de : `region`, `full`, `window`, `last`, `delayed`, `video`, `show`, `hide`, `settings`, `cancel` ou `quit`. Pratique pour un raccourci personnalisé ou un script.
## En cas de problème
- Si l'application ne répond plus, la relancer depuis le menu Démarrer remplace l'instance bloquée.
- Un journal de diagnostic peut être activé par un triple-clic sur la version, dans « À propos ».
- ScreenCapture est un logiciel libre (licence GPL-3.0) : le code source et les versions sont sur GitHub.
## Sur Mac : autorisation d'enregistrer l'écran
ScreenCapture a besoin de l'autorisation **Enregistrement de l'écran** pour les captures et les vidéos (FFmpeg, lancé par l'application, en dépend aussi). Sans elle, les captures sont **noires** ou ne montrent que le fond d'écran.
- Quand l'autorisation manque, un bandeau rouge s'affiche en haut de la fenêtre avec **Ouvrir les Réglages** et **Relancer**.
- Dans Réglages Système > Confidentialité et sécurité > Enregistrement de l'écran, si ScreenCapture est déjà dans la liste mais que les captures restent noires, supprimer l'entrée avec le bouton `−`, l'ajouter de nouveau avec `+` (ou l'activer), puis relancer l'application.
- Le son du système demande en plus l'autorisation **Enregistrement audio du système uniquement**, dans la même page.
> Depuis macOS Sequoia, le système redemande environ une fois par mois l'autorisation à chaque application qui enregistre l'écran. C'est normal : il suffit d'accepter.
