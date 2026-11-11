# Campagne de validation — fenêtres et miniatures

- Date : 11/11/2026 12:34
- Durée : 156 s
- Environnement : 1 écran(s) : (0, 0) 3440×1440 ; bureau virtuel (0, 0) 3440×1440
- Résultat : **139 / 140 vérifications réussies**


## S01 — Démarrage normal

| Vérification | Résultat | Détail |
|---|---|---|
| fenêtre principale présente | ✅ |  |
| affichée à l'écran | ✅ | (234, 234) 1216×819 |
| bouton dans la barre des tâches | ✅ |  |
| sur un écran branché | ✅ |  |
| aucune fenêtre flottante | ✅ |  |
| interface réactive | ✅ |  |

## S02 — Démarrage dans la zone de notification puis ouverture

| Vérification | Résultat | Détail |
|---|---|---|
| fenêtre principale masquée | ✅ |  |
| pas de bouton dans la barre des tâches | ✅ |  |
| interface réactive (masquée) | ✅ |  |
| affichée après ouverture | ✅ |  |
| bouton dans la barre des tâches | ✅ |  |
| même position et taille qu'au démarrage | ✅ | (312, 312) 1216×819 |

## S03 — Capture de zone depuis l'espace de travail affiché (miniature)

| Vérification | Résultat | Détail |
|---|---|---|
| fenêtre principale masquée pendant la sélection | ✅ |  |
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×1440 (bureau (0, 0) 3440×1440) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| sélection fermée | ✅ |  |
| miniature visible | ✅ | (3088, 1094) 336×282 |
| miniature dans la zone de travail | ✅ | zone (0, 0) 3440×1392 |
| miniature en bas à droite | ✅ |  |
| miniature de taille raisonnable | ✅ | 336×282 |
| miniature sans bouton dans la barre des tâches | ✅ |  |
| fenêtre principale réaffichée | ✅ |  |
| fenêtre principale : même position et taille | ✅ | (26, 26) 1216×819 (avant (26, 26) 1216×819) |
| capture enregistrée | ✅ | 1 fichier(s) |
| miniature disparue après 6 s | ✅ |  |
| après la miniature : même position et taille | ✅ | (26, 26) 1216×819 (avant (26, 26) 1216×819) |
| interface réactive | ✅ |  |

## S04 — Sélection annulée avec Échap

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×1440 (bureau (0, 0) 3440×1440) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| sélection fermée | ✅ |  |
| fenêtre principale réaffichée | ✅ |  |
| fenêtre principale : même position et taille | ✅ | (156, 156) 1216×819 (avant (156, 156) 1216×819) |
| aucune miniature | ✅ |  |

## S05 — Capture alors que l'application est dans la zone de notification

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×1440 (bureau (0, 0) 3440×1440) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| fenêtre principale toujours masquée | ✅ |  |
| miniature disparue | ✅ |  |
| fenêtre principale toujours masquée après la miniature | ✅ |  |
| toujours sans bouton dans la barre des tâches | ✅ |  |
| à la réouverture : même position et taille | ✅ | (260, 260) 1216×819 (avant (260, 260) 1216×819) |

## S06 — Capture alors que l'espace de travail est réduit dans la barre des tâches

| Vérification | Résultat | Détail |
|---|---|---|
| fenêtre principale toujours réduite pendant la sélection | ✅ |  |
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×1440 (bureau (0, 0) 3440×1440) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| toujours réduite pendant la miniature | ✅ |  |
| toujours réduite après la miniature | ✅ |  |
| bouton toujours dans la barre des tâches | ✅ |  |
| restaurée à l'ouverture | ✅ |  |
| à la restauration : même position et taille | ✅ | (26, 26) 1216×819 (avant (26, 26) 1216×819) |

## S07 — Trois captures rapprochées pendant la miniature (fenêtre réduite)

| Vérification | Résultat | Détail |
|---|---|---|
| capture n°1 : fenêtre principale toujours réduite | ✅ |  |
| capture n°1 : une seule miniature | ✅ | 1 |
| capture n°2 : fenêtre principale toujours réduite | ✅ |  |
| capture n°2 : une seule miniature | ✅ | 1 |
| capture n°3 : fenêtre principale toujours réduite | ✅ |  |
| capture n°3 : une seule miniature | ✅ | 1 |
| fenêtre principale toujours réduite à la fin | ✅ |  |
| trois captures enregistrées | ✅ | 6 |
| à la restauration : même position et taille | ✅ | (156, 156) 1216×819 (avant (156, 156) 1216×819) |

## S08 — Capture avec ouverture dans l'éditeur

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×1440 (bureau (0, 0) 3440×1440) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| éditeur affiché | ✅ | Capture_2026-11-11_12-35-15.png — ScreenCapture |
| fenêtre de l'éditeur : même position et taille | ✅ | (26, 26) 1216×819 (avant (26, 26) 1216×819) |
| au premier plan | ✅ | Capture_2026-11-11_12-35-15.png — ScreenCapture |

## S09 — Éditeur : Copier puis nouvelle capture avec Impr. écran (blocage signalé)

| Vérification | Résultat | Détail |
|---|---|---|
| éditeur ouvert | ✅ |  |
| essai 1 : sélection affichée après Copier + Impr. écran | ✅ |  |
| essai 1 : interface réactive | ✅ |  |
| essai 1 : éditeur réaffiché | ✅ |  |
| essai 2 : sélection affichée après Copier + Impr. écran | ✅ |  |
| essai 2 : interface réactive | ✅ |  |
| essai 2 : éditeur réaffiché | ✅ |  |
| essai 3 : sélection affichée après Copier + Impr. écran | ✅ |  |
| essai 3 : interface réactive | ✅ |  |
| essai 3 : éditeur réaffiché | ✅ |  |
| essai 4 : sélection affichée après Copier + Impr. écran | ✅ |  |
| essai 4 : interface réactive | ✅ |  |
| essai 4 : éditeur réaffiché | ✅ |  |
| essai 5 : sélection affichée après Copier + Impr. écran | ✅ |  |
| essai 5 : interface réactive | ✅ |  |
| essai 5 : éditeur réaffiché | ✅ |  |
| fenêtre de l'éditeur : même position et taille | ✅ | (130, 130) 1216×819 (avant (130, 130) 1216×819) |

## S10 — Captures plein écran, fenêtre et dernière zone

| Vérification | Résultat | Détail |
|---|---|---|
| plein écran : capture enregistrée | ✅ | 9 fichier(s) |
| plein écran : fenêtre principale réaffichée | ✅ |  |
| plein écran : même position et taille | ✅ | (338, 338) 1216×819 (avant (338, 338) 1216×819) |
| dernière zone : capture enregistrée | ✅ | 10 fichier(s) |
| dernière zone : fenêtre principale réaffichée | ✅ |  |
| dernière zone : même position et taille | ✅ | (338, 338) 1216×819 (avant (338, 338) 1216×819) |
| fenêtre : capture enregistrée | ✅ | 11 fichier(s) |
| fenêtre : fenêtre principale réaffichée | ✅ |  |
| fenêtre : même position et taille | ✅ | (338, 338) 1216×819 (avant (338, 338) 1216×819) |

## S11 — Ouverture depuis la zone de notification pendant la sélection

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×1440 (bureau (0, 0) 3440×1440) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| sélection fermée | ✅ |  |
| fenêtre principale affichée | ✅ |  |
| fenêtre principale : même position et taille | ✅ | (156, 156) 1216×819 (avant (156, 156) 1216×819) |
| nouvelle capture possible ensuite | ✅ |  |

## S12 — Fermeture par la croix puis réouverture

| Vérification | Résultat | Détail |
|---|---|---|
| masquée (zone de notification) | ✅ |  |
| plus de bouton dans la barre des tâches | ✅ |  |
| processus toujours actif | ✅ |  |
| réaffichée | ✅ |  |
| à la réouverture : même position et taille | ✅ | (286, 286) 1216×819 (avant (286, 286) 1216×819) |

## S13 — Impr. écran pendant la sélection

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×1440 (bureau (0, 0) 3440×1440) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| toujours une seule sélection, visible | ✅ | 1 |
| interface réactive | ✅ |  |
| sélection fermée | ✅ |  |

## S14 — Fenêtre restée sur un écran débranché

| Vérification | Résultat | Détail |
|---|---|---|
| ramenée sur un écran branché | ✅ | (1112, 286) 1216×819 |
| affichée | ✅ |  |

## S15 — Endurance : 20 captures et annulations enchaînées

| Vérification | Résultat | Détail |
|---|---|---|
| aucune sélection manquée ni fenêtre figée | ✅ | 0 échec(s) |
| interface réactive | ✅ |  |
| fenêtre principale : même position et taille | ✅ | (208, 208) 1216×819 (avant (208, 208) 1216×819) |
| fenêtre principale affichée | ✅ |  |

## S16 — Film : la fenêtre réduite n'apparaît jamais pendant deux captures

| Vérification | Résultat | Détail |
|---|---|---|
| l'espace de travail se distingue du bureau | ✅ |  |
| aucune image du film ne montre la fenêtre principale | ✅ | 0 image(s) sur ~960 |
| toujours réduite | ✅ |  |
| contrôle : le détecteur voit la fenêtre quand elle est affichée | ✅ | 247 image(s) |

## S17 — Enregistrement vidéo : barre flottante

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×1440 (bureau (0, 0) 3440×1440) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| barre d'enregistrement visible | ✅ | (1540, 528) 360×60 |
| barre hors de la zone filmée | ✅ |  |
| fenêtre principale masquée pendant l'enregistrement | ✅ |  |
| barre fermée | ✅ |  |
| fenêtre principale réaffichée | ✅ |  |
| fenêtre principale : même position et taille | ✅ | (26, 26) 1216×819 (avant (26, 26) 1216×819) |
| vidéo enregistrée | ✅ | 1 |

## S18 — Plusieurs écrans : fenêtre sur chaque écran, sélection et miniature

| Vérification | Résultat | Détail |
|---|---|---|
| au moins deux écrans branchés | ❌ | un seul écran : scénario non exécuté |

## Captures d'écran

- S03 — Capture de zone depuis l'espace de travail affiché (miniature) : `S03_selection.png`
- S03 — Capture de zone depuis l'espace de travail affiché (miniature) : `S03_miniature.png`
- S08 — Capture avec ouverture dans l'éditeur : `S08_editeur.png`
