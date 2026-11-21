# Willkommen
ScreenCapture nimmt den Bildschirm auf, beschriftet Bilder und zeichnet Videos auf. Fast jede Aktion lässt sich auf mehrere Arten ausführen: Tastenkürzel, Schaltfläche auf der Startseite, Menü im Infobereich oder Maus. Diese Hilfe stellt alle vor.
![accueil]
- **Start**: eine Aufnahme starten und den Verlauf durchsuchen.
- **Einstellungen**: Ordner, Format, Tastenkürzel, Aufbewahrung, Sprache, Updates.
- **Hilfe**: diese Seite. Taste `F1` auf der Startseite.
> ScreenCapture bleibt im Infobereich (neben der Uhr) verfügbar: Das Schließen-Kreuz blendet das Fenster aus, ohne die Anwendung zu beenden.
> Auf dem Mac entspricht `Strg` der Taste `⌘`; die Aufnahme-Tastenkürzel werden in den Einstellungen gewählt.

# Eine Aufnahme machen
Sechs Aufnahmearten:
- **Bereich**: ein Teil des Bildschirms, mit der Maus gewählt.
- **Vollbild**: alle Bildschirme auf einmal.
- **Fenster**: das Fenster im Vordergrund.
- **Letzter Bereich**: derselbe Bereich wie bei der vorherigen Aufnahme, ohne ihn neu zu ziehen.
- **Verzögert**: ein Bereich nach einem Countdown (zum Beispiel, um ein Menü zu öffnen).
- **Video**: Aufzeichnung eines Bereichs, siehe „Ein Video aufnehmen“.
## Eine Aufnahme starten
- Das **Tastenkürzel**, aus jeder Anwendung: `Druck` für einen Bereich, `Strg + Druck` für das Vollbild, `Alt + Druck` für ein Fenster, `Umschalt + Druck` für den letzten Bereich, `Strg + Umschalt + Druck` für ein Video. Sie lassen sich in den Einstellungen ändern.
- Die **Karten der Startseite** (Bereich, Vollbild, Fenster…).
- Das **Menü im Infobereich**: Rechtsklick auf das ScreenCapture-Symbol.
- Die **Schaltfläche der linken Leiste** (oben), für einen Bereich.
- Die **Befehlszeile**: `ScreenCapture.exe --cmd region` (siehe „Weiterführendes“).
## Den Bereich wählen
![selection]
- **Ziehen**, um den Bereich zu zeichnen; beim Loslassen wird aufgenommen.
- Auf ein Fenster **klicken**, um es ganz aufzunehmen (es wird beim Überfahren umrahmt).
- `Eingabe` nimmt den ganzen Bildschirm auf, `Esc` bricht ab.
- `C` kopiert die Farbe unter dem Mauszeiger (Hex-Code); die Lupe zeigt vergrößerte Pixel.
- Die Hilfeleiste weicht von selbst aus, wenn sie den ausgewählten Bereich verdeckt.
## Nach der Aufnahme
Je nach Einstellung „Nach der Aufnahme“:
- **Miniatur**: Eine Miniatur erscheint für einige Sekunden. Ein Klick darauf öffnet den Editor.
- **Editor**: Das Bild öffnet sich direkt zum Beschriften.
- **Nichts**: Die Aufnahme wird nur gespeichert und/oder kopiert.
Die Aufnahme kann außerdem automatisch in die Zwischenablage kopiert und im Aufnahmeordner gespeichert werden (Einstellungen).
## Scroll-Aufnahme
Um eine Seite aufzunehmen, die höher als der Bildschirm ist (Webseite, Dokument…): Karte **Scrollen** auf der Startseite oder Menü im Infobereich.
- Den scrollenden Bereich wählen, möglichst ohne Bildlaufleiste.
- Die Seite scrollt von selbst nach unten und die Bilder werden zusammengesetzt; die Aufnahme endet am Seitenende oder mit **Fertig** / `Esc`.
- Feste Kopf- und Fußzeilen werden nur einmal übernommen.
> Auf dem Mac braucht das automatische Scrollen die Berechtigung **Bedienungshilfen** (Systemeinstellungen > Datenschutz & Sicherheit). Ohne sie die Seite selbst mit Mausrad oder Trackpad scrollen, dann **Fertig**.

# Im Editor beschriften
![editeur]
Der Editor öffnet sich nach einer Aufnahme, per Doppelklick auf eine Miniatur im Verlauf, über die Schaltfläche „Bild im Editor öffnen“ oder wenn Sie eine Bilddatei auf das Fenster ziehen.
## Die Werkzeuge
Jedes Werkzeug hat eine Taste (ohne Strg):
- `V` Auswahl, `A` Pfeil, `L` Linie, `R` Rechteck, `E` Ellipse, `P` Freihand.
- `H` Textmarker, `B` Weichzeichnen / Verpixeln, `M` Lupe.
- `T` Text, `U` Sprechblase, `N` Nummerierung, `S` Stempel.
- `C` Zuschneiden, `X` Streifen herausschneiden, `D` Bereich verschieben, `I` Pipette.
Wenn Sie beim Zeichnen `Umschalt` gedrückt halten, entstehen 45°-Winkel (Pfeile, Linien) oder regelmäßige Formen (Quadrat, Kreis).
## Eine Form bearbeiten
- **Auswählen**: Werkzeug Auswahl, dann auf die Form klicken. Mit den Griffen ändern Sie ihre Größe.
- **Verschieben**: die Form ziehen oder die Pfeiltasten verwenden (`Umschalt` + Pfeil: 10 Pixel).
- **Im vergrößerten Bild bewegen**: die **rechte Maustaste** (oder die mittlere) gedrückt halten und die Maus bewegen; das Mausrad scrollt ebenfalls.
- **Sprechblasen**: vier Typen (abgerundet, rechteckig, oval, Gedanke) im rechten Bereich. Den **Griff der Spitze** ziehen, um sie auszurichten oder zu verlängern, ohne die Blase zu verschieben; „Standardspitze“ setzt sie zurück.
- **Duplizieren**: `Strg + D` oder die Schaltfläche „Duplizieren“ im rechten Bereich. Eine duplizierte Nummer erhält die nächste Nummer der Reihe.
- **Inhalt ändern**: **Doppelklick** auf einen Text, eine Sprechblase, eine Nummer oder einen Stempel (oder Schaltfläche „Inhalt ändern“). Bei einer Nummer den neuen Wert eingeben (5, e, IV… je nach Stil) und `Eingabe` drücken.
- **Farbe, Stärke, Textgröße, Füllung, Schatten, gestrichelt**: rechter Bereich, angewendet auf die ausgewählte Form oder auf die nächsten Formen, wenn nichts ausgewählt ist.
- **In den Vordergrund / in den Hintergrund** und **Löschen** (`Entf`): Schaltflächen im rechten Bereich.
- **Rückgängig / Wiederholen**: `Strg + Z` / `Strg + Y`.
## Nummern und Stempel
- **Nummerierung**: Jeder Klick setzt die nächste Nummer. Im rechten Bereich wählen Sie den Stil (1, 2, 3 · a, b, c · I, II, III…) und den nächsten Wert und können alles der Reihe nach neu nummerieren.
- **Stempel**: GENEHMIGT, VERTRAULICH, ENTWURF… oder Ihr eigener Text im Feld **Eigener Stempel** (er wird gespeichert). Ein Klick auf das Bild setzt den Stempel; ein Doppelklick auf einen gesetzten Stempel ändert seinen Text.
## Das Bild umwandeln
- **Zuschneiden** (`C`): den zu behaltenden Bereich ziehen, dann `Eingabe`.
- **Streifen herausschneiden** (`X`): entfernt einen waagerechten oder senkrechten Streifen und fügt die beiden Teile wieder zusammen (gerade Naht oder Zickzack).
- **Bereich verschieben** (`D`): einen Teil des Bildes auswählen und ziehen; die ursprüngliche Stelle wird mit der Hintergrundfarbe gefüllt.
- **Drehen**, **Größe ändern** und **Text erkennen** (Zeichenerkennung): Schaltflächen in der oberen Leiste.
## Speichern und teilen
- `Strg + S` speichert, `Strg + Umschalt + S` speichert unter einem anderen Namen oder Format.
- `Strg + C` kopiert das beschriftete Bild in die Zwischenablage.
- **Exporteffekte** (Rahmen, Schlagschatten): werden dem gespeicherten oder kopierten Bild hinzugefügt.
- `Strg + Mausrad` zoomt, `Strg + 0` passt an das Fenster an; die mittlere Maustaste verschiebt die Ansicht.

# Der Aufnahmeverlauf
Die Startseite listet die Aufnahmen des Ordners auf, die neuesten zuerst.
- **Filter** Alle / Bilder / Videos und **Suche** nach Namen.
- **Öffnen**: Doppelklick (Editor für ein Bild, Player für ein Video).
- **Schnellaktionen** beim Überfahren einer Miniatur: bearbeiten, kopieren, anheften, im Explorer anzeigen, löschen. Rechtsklick: vollständiges Menü.
## Mehrere Aufnahmen auswählen
- **Das runde Kästchen** oben links auf einer Miniatur fügt sie mit einem einfachen Klick zur Auswahl hinzu. Danach fügt ein Klick auf eine andere Miniatur diese hinzu oder entfernt sie, ohne eine Taste zu halten.
- Oder: `Strg` + Klick zum Hinzufügen oder Entfernen, `Umschalt` + Klick für einen Bereich, `Strg + A` für alle.
- Die **Auswahlleiste** zeigt die Anzahl der Aufnahmen und ermöglicht es, alle auszuwählen, anzuheften oder **die ganze Auswahl** mit einem Klick zu **löschen** (oder `Entf`). `Esc` hebt die Auswahl auf.
- Gelöschte Aufnahmen kommen nach einer Bestätigung in den Papierkorb.
## Bilder hinzufügen
- `Strg + V` fügt das Bild aus der Zwischenablage in den Aufnahmeordner ein.
- Bilddateien aus dem Explorer auf das Fenster ziehen fügt sie hinzu.
- Die Schaltfläche „Bild im Editor öffnen“ öffnet jedes beliebige Bild im Editor.

# Ein Video aufnehmen
- Starten: `Strg + Umschalt + Druck`, die Karte **Video** auf der Startseite oder das Menü im Infobereich, dann den Bereich wie bei einer Aufnahme wählen.
- **Ton**: Die Taste `M` (oder ein Klick auf das Etikett) wählt vor dem Start mit oder ohne Mikrofon. Während der Aufnahme schaltet die Mikrofon-Schaltfläche der schwebenden Leiste den Ton stumm oder wieder ein.
- **Stoppen**: Schaltfläche der schwebenden Leiste oder dasselbe Tastenkürzel. **Abbrechen**: `Esc`.
- Formate MP4, WebM oder GIF, Bilder pro Sekunde, Mauszeiger und Mikrofon: in den Einstellungen. Die Aufnahme verwendet FFmpeg (vom Installationsprogramm angeboten).

# Aufbewahrung der Aufnahmen
In den Einstellungen kann ScreenCapture alte Aufnahmen automatisch löschen:
- eine **Dauer** für Bilder und eine andere für Videos (7, 30 oder 90 Tage, 1 Jahr, eigene Dauer oder nie);
- eine **maximale Ordnergröße**: darüber hinaus werden die ältesten zuerst gelöscht;
- in den **Papierkorb** (wiederherstellbar) oder **endgültig**.
**Angeheftete** Aufnahmen (Nadel auf der Miniatur) und die im Editor geöffnete werden nie gelöscht; Dateien, die nicht von ScreenCapture stammen, werden nie angetastet. „Jetzt aufräumen“ zeigt vorher, was gelöscht wird.

# Einstellungen
![parametres]
- **Aufnahme**: Ordner, Format PNG oder JPEG, automatisches Speichern und Kopieren, was nach einer Aufnahme geschieht, Mauszeiger, Verzögerung der verzögerten Aufnahme, Lupe.
- **Tastenkürzel**: ein Kürzel eingeben (zum Beispiel `ctrl+shift+KeyS`) und „Übernehmen“; leer lassen zum Deaktivieren.
- **Editorfarben**: die angebotenen Farben ändern, hinzufügen oder entfernen.
- **Updates**: automatische Suche und Schaltfläche „Jetzt suchen“.
- **Darstellung und Start**: Sprache (Französisch, Englisch, Spanisch, Deutsch oder die des Systems), helles oder dunkles Design, Akzentfarbe, Schließen in den Infobereich, minimiert starten.
- **Beim Windows-Start ausführen** (bzw. bei der Anmeldung auf dem Mac): jederzeit ein- oder ausschaltbar, auch wenn es bei der Installation gewählt wurde. Der Schalter zeigt den tatsächlichen Zustand.

# Updates und portable Version
## Updates
Wenn eine neue Version veröffentlicht wird, erscheint oben im Fenster eine Leiste. **Aktualisieren** lädt sie herunter, prüft ihren Fingerabdruck, installiert sie und startet ScreenCapture neu. **Später** blendet die Leiste bis zum nächsten Start aus. Auf dem Mac wird das Disk-Image heruntergeladen und geöffnet: Ziehen Sie die Anwendung einfach in „Programme“.
## Portable Version
Die portable Version (`.zip`-Datei) läuft ohne Installation, zum Beispiel von einem USB-Stick: den Ordner entpacken und `ScreenCapture.exe` starten. Einstellungen und Protokoll werden im Ordner `data` neben dem Programm gespeichert. Updates ersetzen das Programm am selben Ort.

# Weiterführendes
## Befehlszeile
Eine laufende Instanz lässt sich mit `ScreenCapture.exe --cmd` steuern, gefolgt von: `region`, `full`, `window`, `last`, `delayed`, `video`, `show`, `hide`, `settings`, `cancel` oder `quit`. Praktisch für ein eigenes Tastenkürzel oder ein Skript.
## Bei Problemen
- Reagiert die Anwendung nicht mehr, ersetzt ein erneuter Start über das Startmenü die blockierte Instanz.
- Ein Diagnoseprotokoll lässt sich per Dreifachklick auf die Version auf der Seite „Info“ aktivieren.
- ScreenCapture ist freie Software (Lizenz GPL-3.0): Quellcode und Versionen liegen auf GitHub.
## Auf dem Mac: Berechtigung zur Bildschirmaufnahme
ScreenCapture benötigt die Berechtigung **Bildschirmaufnahme** für Screenshots und Videos (auch das von der Anwendung gestartete FFmpeg hängt davon ab). Ohne sie sind Screenshots **schwarz** oder zeigen nur den Hintergrund.
- Fehlt die Berechtigung, erscheint oben im Fenster ein rotes Band mit **Einstellungen öffnen** und **Neu starten**.
- Ist ScreenCapture unter Systemeinstellungen > Datenschutz & Sicherheit > Bildschirmaufnahme bereits aufgeführt, die Screenshots bleiben aber schwarz: den Eintrag mit `−` entfernen, mit `+` erneut hinzufügen (oder aktivieren) und die Anwendung neu starten.
- Der Systemton braucht zusätzlich die Berechtigung **Nur Systemaudioaufnahme** auf derselben Seite.
> Seit macOS Sequoia fragt das System etwa einmal im Monat erneut bei jeder Anwendung nach, die den Bildschirm aufnimmt. Das ist normal: einfach zustimmen.
