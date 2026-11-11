# Campagne de validation — fenêtres et miniatures

- Date : 11/11/2026 14:49
- Durée : 326 s
- Environnement : 2 écran(s) : (0, 0) 3440×1440, (609, 1440) 2160×1440 ; bureau virtuel (0, 0) 3440×2880
- Résultat : **241 / 241 vérifications réussies**


## S01 — Démarrage normal

| Vérification | Résultat | Détail |
|---|---|---|
| fenêtre principale présente | ✅ |  |
| affichée à l'écran | ✅ | (182, 182) 1216×819 |
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
| même position et taille qu'au démarrage | ✅ | (260, 260) 1216×819 |

## S03 — Capture de zone depuis l'espace de travail affiché (miniature)

| Vérification | Résultat | Détail |
|---|---|---|
| fenêtre principale masquée pendant la sélection | ✅ |  |
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×2880 (bureau (0, 0) 3440×2880) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| sélection fermée | ✅ |  |
| miniature visible | ✅ | (3088, 1094) 336×282 |
| miniature dans la zone de travail | ✅ | zone (0, 0) 3440×1392 |
| miniature en bas à droite | ✅ |  |
| miniature de taille raisonnable | ✅ | 336×282 |
| miniature sans bouton dans la barre des tâches | ✅ |  |
| fenêtre principale réaffichée | ✅ |  |
| fenêtre principale : même position et taille | ✅ | (338, 338) 1216×819 (avant (338, 338) 1216×819) |
| capture enregistrée | ✅ | 1 fichier(s) |
| miniature disparue après 6 s | ✅ |  |
| après la miniature : même position et taille | ✅ | (338, 338) 1216×819 (avant (338, 338) 1216×819) |
| interface réactive | ✅ |  |

## S04 — Sélection annulée avec Échap

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×2880 (bureau (0, 0) 3440×2880) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| sélection fermée | ✅ |  |
| fenêtre principale réaffichée | ✅ |  |
| fenêtre principale : même position et taille | ✅ | (104, 104) 1216×819 (avant (104, 104) 1216×819) |
| aucune miniature | ✅ |  |

## S05 — Capture alors que l'application est dans la zone de notification

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×2880 (bureau (0, 0) 3440×2880) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| fenêtre principale toujours masquée | ✅ |  |
| miniature disparue | ✅ |  |
| fenêtre principale toujours masquée après la miniature | ✅ |  |
| toujours sans bouton dans la barre des tâches | ✅ |  |
| à la réouverture : même position et taille | ✅ | (208, 208) 1216×819 (avant (208, 208) 1216×819) |

## S06 — Capture alors que l'espace de travail est réduit dans la barre des tâches

| Vérification | Résultat | Détail |
|---|---|---|
| fenêtre principale toujours réduite pendant la sélection | ✅ |  |
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×2880 (bureau (0, 0) 3440×2880) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| toujours réduite pendant la miniature | ✅ |  |
| toujours réduite après la miniature | ✅ |  |
| bouton toujours dans la barre des tâches | ✅ |  |
| restaurée à l'ouverture | ✅ |  |
| à la restauration : même position et taille | ✅ | (338, 338) 1216×819 (avant (338, 338) 1216×819) |

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
| à la restauration : même position et taille | ✅ | (104, 104) 1216×819 (avant (104, 104) 1216×819) |

## S08 — Capture avec ouverture dans l'éditeur

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×2880 (bureau (0, 0) 3440×2880) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| éditeur affiché | ✅ | Capture_2026-11-11_14-50-39.png — ScreenCapture |
| fenêtre de l'éditeur : même position et taille | ✅ | (338, 338) 1216×819 (avant (338, 338) 1216×819) |
| au premier plan | ✅ | Capture_2026-11-11_14-50-39.png — ScreenCapture |

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
| fenêtre de l'éditeur : même position et taille | ✅ | (78, 78) 1216×819 (avant (78, 78) 1216×819) |

## S10 — Captures plein écran, fenêtre et dernière zone

| Vérification | Résultat | Détail |
|---|---|---|
| plein écran : capture enregistrée | ✅ | 9 fichier(s) |
| plein écran : fenêtre principale réaffichée | ✅ |  |
| plein écran : même position et taille | ✅ | (286, 286) 1216×819 (avant (286, 286) 1216×819) |
| dernière zone : capture enregistrée | ✅ | 10 fichier(s) |
| dernière zone : fenêtre principale réaffichée | ✅ |  |
| dernière zone : même position et taille | ✅ | (286, 286) 1216×819 (avant (286, 286) 1216×819) |
| fenêtre : capture enregistrée | ✅ | 11 fichier(s) |
| fenêtre : fenêtre principale réaffichée | ✅ |  |
| fenêtre : même position et taille | ✅ | (286, 286) 1216×819 (avant (286, 286) 1216×819) |

## S11 — Ouverture depuis la zone de notification pendant la sélection

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×2880 (bureau (0, 0) 3440×2880) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| sélection fermée | ✅ |  |
| fenêtre principale affichée | ✅ |  |
| fenêtre principale : même position et taille | ✅ | (104, 104) 1216×819 (avant (104, 104) 1216×819) |
| nouvelle capture possible ensuite | ✅ |  |

## S12 — Fermeture par la croix puis réouverture

| Vérification | Résultat | Détail |
|---|---|---|
| masquée (zone de notification) | ✅ |  |
| plus de bouton dans la barre des tâches | ✅ |  |
| processus toujours actif | ✅ |  |
| réaffichée | ✅ |  |
| à la réouverture : même position et taille | ✅ | (234, 234) 1216×819 (avant (234, 234) 1216×819) |

## S13 — Impr. écran pendant la sélection

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×2880 (bureau (0, 0) 3440×2880) |
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
| fenêtre principale : même position et taille | ✅ | (130, 130) 1216×819 (avant (130, 130) 1216×819) |
| fenêtre principale affichée | ✅ |  |

## S16 — Film : la fenêtre réduite n'apparaît jamais pendant deux captures

| Vérification | Résultat | Détail |
|---|---|---|
| l'espace de travail se distingue du bureau | ✅ |  |
| aucune image du film ne montre la fenêtre principale | ✅ | 0 image(s) sur ~960 |
| toujours réduite | ✅ |  |
| contrôle : le détecteur voit la fenêtre quand elle est affichée | ✅ | 246 image(s) |

## S17 — Enregistrement vidéo : barre flottante

| Vérification | Résultat | Détail |
|---|---|---|
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×2880 (bureau (0, 0) 3440×2880) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| barre d'enregistrement visible | ✅ | (1540, 528) 360×60 |
| barre hors de la zone filmée | ✅ |  |
| fenêtre principale masquée pendant l'enregistrement | ✅ |  |
| barre fermée | ✅ |  |
| fenêtre principale réaffichée | ✅ |  |
| fenêtre principale : même position et taille | ✅ | (312, 312) 1216×819 (avant (312, 312) 1216×819) |
| vidéo enregistrée | ✅ | 1 |

## S18 — Plusieurs écrans : fenêtre sur chaque écran, sélection et miniature

| Vérification | Résultat | Détail |
|---|---|---|
| écran 1 : fenêtre placée sur l'écran | ✅ | (40, 40) 1216×819 |
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×2880 (bureau (0, 0) 3440×2880) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| écran 1 : miniature sur l'écran du pointeur | ✅ | (3088, 1094) 336×282 (zone (0, 0) 3440×1392) |
| écran 1 : taille de la miniature à l'échelle de l'écran (100 %) | ✅ | 336×282 attendu 336×282 |
| écran 1 : fenêtre principale : même position et taille | ✅ | (40, 40) 1216×819 (avant (40, 40) 1216×819) |
| écran 1 : fenêtre principale toujours sur son écran | ✅ |  |
| écran 2 : fenêtre placée sur l'écran | ✅ | (651, 1480) 1518×1022 |
| sélection visible | ✅ |  |
| sélection couvre tous les écrans | ✅ | (0, 0) 3440×2880 (bureau (0, 0) 3440×2880) |
| sélection sans bouton dans la barre des tâches | ✅ |  |
| sélection au premier plan | ✅ | ScreenCapture — Sélection |
| écran 2 : miniature sur l'écran du pointeur | ✅ | (2329, 2448) 420×352 (zone (609, 1440) 2160×1380) |
| écran 2 : taille de la miniature à l'échelle de l'écran (125 %) | ✅ | 420×352 attendu 420×352 |
| écran 2 : fenêtre principale : même position et taille | ✅ | (651, 1480) 1518×1022 (avant (651, 1480) 1518×1022) |
| écran 2 : fenêtre principale toujours sur son écran | ✅ |  |

## S19 — Espace de travail déplacé à la souris sur le second écran, capture sur cet écran

| Vérification | Résultat | Détail |
|---|---|---|
| échelle du second écran | ✅ | 125 % |
| déplacée sur le second écran (centre de la fenêtre) | ✅ | (507, 1452) 1518×1022 (avant (338, 338) 1216×819) |
| taille adaptée à l'échelle (×1.25), sans rétrécir | ✅ | largeur 1216 → 1518 (attendu ≈ 1520) |
| capture sur le second écran : taille de l'image = zone sélectionnée | ✅ | 500×320 pour 500×320 |
| capture sur le second écran : contenu identique à l'écran | ✅ | ressemblance 100.0 % |
| miniature sur le second écran | ✅ | (2329, 2448) 420×352 |
| miniature à l'échelle 125 % | ✅ | 420×352 attendu 420×352 |
| espace de travail resté sur le second écran | ✅ | (507, 1452) 1518×1022 (avant (507, 1452) 1518×1022) |
| espace de travail affiché | ✅ |  |
| capture sur le premier écran : taille de l'image = zone sélectionnée | ✅ | 500×320 pour 500×320 |
| capture sur le premier écran : contenu identique à l'écran | ✅ | ressemblance 100.0 % |
| espace de travail toujours sur le second écran | ✅ |  |
| rouvert au même endroit sur le second écran | ✅ |  |
| ramenée sur le premier écran (centre de la fenêtre) | ✅ | (-95, 49) 1216×819 |
| taille d'origine retrouvée | ✅ | 1216×819 (départ 1216×819) |

## S20 — Espace de travail agrandi (plein écran) sur le second écran

| Vérification | Résultat | Détail |
|---|---|---|
| agrandie | ✅ |  |
| occupe le second écran | ✅ | (600, 1431) 2178×1398 (zone (609, 1440) 2160×1380) |
| masquée pendant la sélection | ✅ |  |
| toujours agrandie après la capture | ✅ |  |
| même position et taille | ✅ | (600, 1431) 2178×1398 (avant (600, 1431) 2178×1398) |
| plein écran : toujours agrandie | ✅ | (600, 1431) 2178×1398 |
| rouverte agrandie au même endroit | ✅ | (600, 1431) 2178×1398 |
| reste réduite pendant une capture | ✅ |  |
| restaurée agrandie au même endroit | ✅ | (600, 1431) 2178×1398 |
| interface réactive | ✅ |  |

## S21 — Capture plein écran sur deux écrans d'échelles différentes

| Vérification | Résultat | Détail |
|---|---|---|
| capture enregistrée | ✅ |  |
| image à la taille du bureau virtuel (pixels physiques) | ✅ | 3440×2880 pour 3440×2880 |
| second écran à sa place et à sa taille dans l'image | ✅ | ressemblance 100.0 % |

## S22 — Échap annule une capture différée pendant le compte à rebours

| Vérification | Résultat | Détail |
|---|---|---|
| fenêtre cible de test au premier plan | ✅ | Cible de test ScreenCapture-validation |
| espace de travail masqué pendant le délai | ✅ |  |
| aucune sélection ouverte après le délai | ✅ |  |
| aucune capture enregistrée | ✅ | 37 → 37 |
| aucune miniature | ✅ |  |
| espace de travail réaffiché | ✅ |  |
| espace de travail : même position et taille | ✅ | (78, 78) 1216×819 (avant (78, 78) 1216×819) |
| Échap ferme la sélection | ✅ |  |
| interface réactive | ✅ |  |

## S23 — Taille minimale de l'espace de travail (900 × 600 points)

| Vérification | Résultat | Détail |
|---|---|---|
| écran à 100 % : taille minimale respectée | ✅ | 916×639 (minimum 900×600 + bordures) |
| toujours sur le même écran | ✅ |  |
| écran à 125 % : taille minimale respectée | ✅ | 1518×1022 (minimum 1125×750 + bordures) |
| toujours sur le même écran | ✅ |  |

## S24 — Sélection multiple et suppression au clavier dans l'espace de travail

| Vérification | Résultat | Détail |
|---|---|---|
| six vignettes affichées | ✅ | 6 |
| clic puis Ctrl+clic : 2 sélectionnées | ✅ | Some(2) |
| Suppr + Entrée : les 2 captures sélectionnées supprimées | ✅ | restantes : 4 |
| les autres captures conservées | ✅ | ["Test_selection_1.png", "Test_selection_2.png", "Test_selection_3.png", "Test_selection_5.png"] |
| Maj+clic : plage de 3 sélectionnée | ✅ | Some(3) |
| Échap dans la confirmation : rien supprimé | ✅ |  |
| Ctrl+clic sur une capture sélectionnée : retirée | ✅ | Some(2) |
| Ctrl+A : toutes sélectionnées | ✅ | Some(4) |
| Ctrl+A puis Suppr + Entrée : tout supprimé | ✅ | [] |
| interface réactive | ✅ |  |

## S25 — Vidéo avec ou sans le son (bouton / touche M dans la sélection)

| Vérification | Résultat | Détail |
|---|---|---|
| micro disponible pour le test | ✅ | Microphone (MPOW HC6) |
| par défaut : avec le son | ✅ | enregistrement vidéo Rect { x: 1520, y: 600, w: 400, h: 240 } : avec le son du micro « Microphone (MPOW HC6) » |
| la vidéo contient une piste audio | ✅ | Some(1) |
| le son du micro est enregistré | ✅ | Some(-43.4) dB |
| vidéo enregistrée | ✅ |  |
| M : sans le son | ✅ | enregistrement vidéo Rect { x: 1520, y: 600, w: 400, h: 240 } : sans le son (micro « Microphone (MPOW HC6) » coupé, activable pendant l'enregistrement) |
| piste audio silencieuse (micro coupé) | ✅ | Some(-91.0) dB |
| choix « sans le son » mémorisé | ✅ |  |
| M de nouveau : avec le son | ✅ |  |

## S26 — Micro activé puis coupé pendant l'enregistrement (bouton de la barre)

| Vérification | Résultat | Détail |
|---|---|---|
| enregistrement démarré sans le son | ✅ |  |
| micro activé pendant l'enregistrement | ✅ |  |
| micro coupé pendant l'enregistrement | ✅ |  |
| silence au début (micro coupé) | ✅ | dB par seconde : -91 -91 -47 -43 -42 -44 -60 -91 -91 -91 |
| son pendant environ 3 s (micro activé) | ✅ | secondes avec du son : [2, 3, 4, 5, 6] |
| silence à la fin (micro recoupé) | ✅ |  |
| choix mémorisé (dernier état : coupé) | ✅ |  |

## S27 — Le bandeau d'aide s'écarte de ce qu'on vise

| Vérification | Résultat | Détail |
|---|---|---|
| pointeur au milieu : bandeau en haut | ✅ | haut |
| pointeur en haut au centre : bandeau déplacé en bas | ✅ | bas |
| sélection sur toute la hauteur : bandeau masqué | ✅ | masqué |
| la sélection commence exactement au point cliqué (y = 5) | ✅ | Some(Rect { x: 1420, y: 5, w: 600, h: 1430 })") / miniature  |

## S28 — Conservation des captures : durée, épinglage, fichiers étrangers, taille maximale

| Vérification | Résultat | Détail |
|---|---|---|
| nettoyage automatique au démarrage | ✅ |  |
| images de plus de 30 jours supprimées (capture et collage) | ✅ | ["Capture_A_3j.png", "Capture_B_10j.png", "Capture_D_100j_epinglee.png", "Notes_personnelles_100j.png", "Video_F_40j.mp4"] |
| images récentes conservées (3 et 10 jours) | ✅ |  |
| capture épinglée conservée (100 jours) | ✅ |  |
| vidéo conservée (durée « Jamais » pour les vidéos) | ✅ |  |
| fichier personnel non concerné (100 jours) | ✅ |  |
| épinglage enregistré | ✅ |  |
| 7 jours : la capture de 10 jours épinglée reste | ✅ | ["Capture_A_3j.png", "Capture_B_10j.png", "Capture_D_100j_epinglee.png", "Notes_personnelles_100j.png"] |
| 7 jours : la capture de 3 jours reste | ✅ |  |
| taille maximale : les 3 plus anciennes supprimées | ✅ | ["Capture_T3_20j.png", "Capture_T4_10j.png"] |
| mode Corbeille : capture ancienne retirée du dossier | ✅ |  |
| journal : suppression signalée | ✅ |  |
| interface réactive | ✅ |  |

## Captures d'écran

- S03 — Capture de zone depuis l'espace de travail affiché (miniature) : `S03_selection.png`
- S03 — Capture de zone depuis l'espace de travail affiché (miniature) : `S03_miniature.png`
- S08 — Capture avec ouverture dans l'éditeur : `S08_editeur.png`
- S19 — Espace de travail déplacé à la souris sur le second écran, capture sur cet écran : `S19_deplacee.png`
- S19 — Espace de travail déplacé à la souris sur le second écran, capture sur cet écran : `S19_miniature.png`
- S20 — Espace de travail agrandi (plein écran) sur le second écran : `S20_agrandie.png`
- S24 — Sélection multiple et suppression au clavier dans l'espace de travail : `S24_selection.png`
- S24 — Sélection multiple et suppression au clavier dans l'espace de travail : `S24_confirmation.png`
- S25 — Vidéo avec ou sans le son (bouton / touche M dans la sélection) : `S25_avec_le_son.png`
- S25 — Vidéo avec ou sans le son (bouton / touche M dans la sélection) : `S25_M_sans_le_son.png`
- S25 — Vidéo avec ou sans le son (bouton / touche M dans la sélection) : `S25_choix_mémorisé.png`
- S25 — Vidéo avec ou sans le son (bouton / touche M dans la sélection) : `S25_M_avec_le_son.png`
- S26 — Micro activé puis coupé pendant l'enregistrement (bouton de la barre) : `S26_barre_micro_coupe.png`
- S26 — Micro activé puis coupé pendant l'enregistrement (bouton de la barre) : `S26_barre_micro_actif.png`
- S27 — Le bandeau d'aide s'écarte de ce qu'on vise : `S27_bandeau_en_bas.png`
- S27 — Le bandeau d'aide s'écarte de ce qu'on vise : `S27_bandeau_masque.png`
