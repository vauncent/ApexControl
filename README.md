# ApexControl — Windows Alpha

Independent, open-source control app for the **wired SteelSeries Apex Pro Mini**. Early alpha for community testing; not affiliated with SteelSeries.

**[Download Windows x64](https://github.com/vincentmittermaier-jpg/ApexControl/releases) · [Report a bug](https://github.com/vincentmittermaier-jpg/ApexControl/issues) · [Changelog](CHANGELOG.md)**

Extract the complete ZIP and run `ApexControl.exe`. Fully exit SteelSeries GG before changing keyboard settings. No administrator rights or GG installation are needed for normal use. The interface supports English and German.

### Supported hardware

| Device | Status |
| --- | --- |
| Wired Apex Pro Mini, USB `1038:161E`, firmware `1.19.7` | Tested development device |
| Other firmware versions and regional hardware variants | Not fully verified |
| Apex Pro TKL, full-size, wireless and other Apex models | Configuration not supported yet |
| Linux | Port planned; no supported Linux release yet |

### Features and alpha limits

Actuation, Rapid Trigger, protection mode, five onboard profile slots, local profile drafts, automatic backups with a restore interface, key remapping, basic keyboard macros, per-key static RGB, breathing, rainbow wave and reactive ripple. Includes tray operation, single-instance startup and optional Windows autostart.

The active onboard slot cannot yet be queried reliably; startup shows the last ApexControl selection without activating a different slot. Macros currently support one basic native macro in a previously macro-free profile, up to 1000 events, without mouse actions or loops. RGB is live-only and separate from saved profiles; software effects require ApexControl to keep running. Layout previews include unverified regional variants. Profile transfers have readback checks and a rollback attempt, but an interrupted write can still require recovery. No firmware updates are provided.

The latest development build passes 48 automated tests. Hardware checks and user feedback cover the wired Mini; physical actuation depths and macro playback have not been independently measured. Please report the exact model, firmware and reproduction steps when testing. See [SECURITY.md](SECURITY.md) for safeguards and remaining limitations.

### Build from source

On Windows, install stable Rust with the MSVC toolchain and Visual Studio Build Tools with **Desktop development with C++**, including a Windows SDK. From the repository folder:

```powershell
cargo test --locked
cargo build --release --locked
```

The executable is `target/release/apexcontrol.exe`. A Windows GNU toolchain is also usable with GCC and `windres`; the local `build.ps1` helper uses the development machine's ignored `.tools` directory. The MSVC instructions are a supported build setup description, not a claim of a separately tested MSVC release binary. No private USB captures are required for ordinary builds or unit tests.

Source and original logo artwork are **GPL-3.0-or-later**. Distributed derivative versions must follow that license and provide corresponding source. Contributions and forks for additional devices are welcome.

---

## Deutsche Bedienungs- und Entwicklungsdokumentation

Native Rust-App mit schlichter dunkler Oberfläche. Entwicklungsfassung 0.1.0 für Windows x64; unabhängig von SteelSeries GG.

Sicherheitsprüfung, Schutzmaßnahmen, Grenzen und Abhängigkeitshinweise sind in [SECURITY.md](SECURITY.md) dokumentiert. Die Prüfung ist keine vollständige Sicherheitszertifizierung.

Stand 2026-11-02: 48 Tests bestehen, einschließlich Startsperre, Backup-Vorschau, Schutz ungespeicherter Profiländerungen, RGB-Auswahl und Auswahlbox. Der Protokollstand stimmt mit den GG-Referenzpaketen überein. Der reine Lesetest an der angeschlossenen Mini mit Firmware 1.19.7 besteht auch mit den verschärften USB-Prüfungen. Die dokumentierten Dateigrößenlimits und die bereits bestehende Grenze von 1000 Ereignissen pro selbst erstelltem Makro gelten weiterhin.

## Start

Oben steht **Updates · 0.1.0**. Ein Klick zeigt die installierte Version und den eingebetteten, scrollbaren Changelog in der gewählten Menüsprache. Die Quelldateien sind [CHANGELOG.md](CHANGELOG.md) und [CHANGELOG.de.md](CHANGELOG.de.md); beide werden auch im Windows-Paket mitgeliefert. Auf Wunsch öffnet sich die geplante GitHub-Releases-Seite im Browser. Der Kanal wird mit der ersten öffentlichen Veröffentlichung eingerichtet; automatische Versionsabfrage, Downloads und Installation sind noch nicht enthalten. Die App meldet deshalb keinen ungeprüften „aktuell“-Status.

Die USB-Anzeige bezieht sich auf die letzte Gerätesuche: **USB bereit** ist grün, wenn ein unterstütztes Gerät ohne Lesefehler ausgelesen wurde; **USB · –** bedeutet kein erkanntes Gerät, **USB · !** einen Such-/Zugriffsfehler. Ein erkanntes, noch nicht konfigurierbares Modell erscheint neutral. Der Hinweis beim Darüberfahren erklärt den Zustand. Nach dem Umstecken **Geräte aktualisieren** verwenden; dies ist keine laufende Hotplug-Überwachung.

`dist/ApexControl.exe` starten. Keine Installation, kein Python und keine GG-Installation für den Gerätezugriff erforderlich. SteelSeries GG vor Änderungen vollständig beenden, da GG parallel eigene Einstellungen senden kann.

Unter Windows läuft die Oberfläche nur einmal pro Benutzer und Anmeldesitzung. Ein weiterer manueller Start zeigt das bereits vorhandene Fenster wieder an, auch aus dem Infobereich. Ein doppelter Windows-Autostart öffnet kein zusätzliches Fenster. Die Prüfung erfolgt vor dem Einlesen der Benutzerdateien und der Gerätesuche; auch gleichzeitig gestartete Kopien an verschiedenen Dateipfaden teilen dieselbe Startsperre. Diese wird beim Prozessende vom Betriebssystem freigegeben. Beim Wechsel von älteren Versionen ohne Startsperre müssen deren laufende Instanzen einmal vollständig beendet werden. Die Diagnoseargumente wie `--offline-check` bleiben separat nutzbar.

Die Startseite zeigt angeschlossene Apex-Tastaturen. Die kabelgebundene **Apex Pro Mini (1038:161E), Firmware 1.19.7** kann konfiguriert werden. Andere Apex-Modelle werden angezeigt, ihre Schreibbefehle sind noch nicht freigeschaltet.

Die App startet immer in der Geräteübersicht. Erst **Öffnen** oder ein Klick auf die Tastatur öffnet den Profil-Editor mit dem zuletzt in ApexControl verwendeten Profil einschließlich lokaler Entwürfe. Die frische Sicherungsgrundlage für Onboard-Schreibvorgänge wird trotzdem vom Gerät gelesen. Ohne vorherige Auswahl wird Slot 1 zur Ansicht geöffnet. Der tatsächlich aktive Slot kann bislang nicht zuverlässig abgefragt werden; die App kennzeichnet das und aktiviert beim Start keinen anderen Slot.

Über das Zahnrad oben rechts lassen sich **Mit Windows starten** und **Beim Schließen in den Infobereich minimieren** einschalten. Beides ist zunächst aus. Zusätzlich kann die App beim Windows-Autostart im Hintergrund starten. Ein Klick oder Doppelklick auf das orange ApexControl-Symbol neben der Uhr (ggf. im Menü unter `^`) öffnet das Fenster wieder. Das Kontextmenü bietet Öffnen, Einstellungen und Beenden. **Beenden** schließt die App unabhängig von der Minimieren-Option vollständig; laufende Geräteaufträge werden zuvor abgewartet. Ohne verfügbares Tray-Symbol wird das Fenster nicht versteckt.

Autostart gilt nur für den aktuellen Windows-Benutzer und wird erst beim Einschalten der Option in dessen `Run`-Registrierung eingetragen. Dafür sind keine Administratorrechte nötig. Er startet bei der Anmeldung die aktuelle portable EXE mit `--autostart`; die EXE muss deshalb am selben Ort bleiben. Nach dem ersten Einschalten bleibt ApexControl im **Task-Manager → Autostart-Apps** sichtbar. Ausschalten setzt den eigenen `StartupApproved\Run`-Wert auf deaktiviert und erhält den registrierten Programmeintrag, damit Windows ihn wieder aktivieren kann. Die App liest beide Werte; bei geöffneten Einstellungen wird der Zustand einmal pro Sekunde aktualisiert. Beim Start und beim Abfragen werden keine Autostartwerte verändert. Die beiden Fensteroptionen werden unter `%LOCALAPPDATA%\ApexControl\settings.json` gespeichert. Ein normaler manueller Start zeigt weiterhin die Geräteübersicht. Hintergrund: [Microsofts Beschreibung der Autostartanzeige](https://learn.microsoft.com/en-us/windows/win32/w8cookbook/startup-apps); die zusätzliche Zustandsauswertung entspricht dem [von Electron verwendeten Mechanismus](https://github.com/electron/electron/blob/main/shell/browser/browser_win.cc).

**Sprache / Language** im Zahnrad-Menü wechselt sofort zwischen **Deutsch** und **English**, einschließlich Tray-Menü, Tastenbelegung, Makro-Editor und Statusmeldungen. Die Auswahl wird in derselben Einstellungsdatei gespeichert. Bestehende Einstellungen ohne Sprachfeld bleiben deutsch. Die Sprache der Oberfläche ändert weder das physische Deutsch-ISO-Tastaturlayout noch eigene Profil- und Makronamen.

Beim ersten Start fragt ein modaler Dialog auf Englisch nach **English** oder **Deutsch**. Er setzt den Tastaturfokus auf die Sprachwahl und zeigt keinen Pfeil. Nach dem Sprachklick erscheint im selben Dialog ein Dropdown für **USA (ANSI)**, **Deutsch (ISO)** sowie Spanisch und UK, bereits in der gewählten Menüsprache. **Fertig** ist erst nach einer Layoutauswahl aktiv und speichert Sprache und Layout gemeinsam; **Zurück** erlaubt eine andere Sprache. Die restliche Oberfläche ist bis zum Abschluss gesperrt. Nach erfolgreichem Speichern werden `language_setup_done` und `layout_setup_done` gesetzt; danach erscheint der Dialog nicht mehr. Bei einem Speicherfehler bleibt die Auswahl offen, bei einem ersten Windows-Autostart wird sie nicht im Hintergrund verborgen. Bestehende Nutzer mit bereits gewählter Sprache und noch ohne Layoutwahl beginnen direkt im zweiten Schritt. Wer schon Sprache und ein manuelles Layout gespeichert hat, behält beide ohne erneute Einrichtung.

Unter **Tastaturlayout** gibt es **Automatisch**, **Deutsch (ISO)**, **USA (ANSI)**, **Spanisch (ISO)**, **UK (ISO)**, **Französisch (AZERTY ISO)**, **Nordisch (ISO)** und **Japanisch (JIS)**. Die sechs vom Nutzer bereitgestellten GG-Screenshots dienen als visuelle Referenz; spanisch bleibt zusätzlich verfügbar. Die Geräteabfragen `F5` und `F2` lesen Region und Bauform. An der angeschlossenen Mini wurden Region 4 und Form 1 zurückgemeldet und damit Deutsch-ISO erkannt. US-ANSI und UK-ISO werden anhand der bekannten Identifikatoren zugeordnet. Weitere, noch nicht zugeordnete Regionen und Bauformen erfordern die manuelle Auswahl. Ohne Erkennung bleibt zunächst die bisherige deutsche Ansicht als Vorschau bestehen. Die Zuordnungen anderer realer Gerätevarianten sind noch nicht physisch getestet.

Das Layout ändert Beschriftungen und Geometrie der Mini (61 ANSI-, 62 ISO- oder 65 JIS-Tasten). Haupt- und Shift-Zeichen folgen dem ausgewählten Layout; die nordische Ansicht zeigt die kombinierten Ø/Ö- und Æ/Ä-Tastenkappen. Französisch berücksichtigt AZERTY einschließlich der M-Position auch in der Belegungsauswahl. JIS hat eine schmale Rücktaste, L-förmiges Enter, eine 5,5 Einheiten breite Leertaste sowie Yen-, Ro-, Muhenkan-, Henkan- und Kana-Tasten. Die bereits in der Mini-Profilstruktur vorhandenen internationalen HID-Positionen werden verwendet; ihre Zuordnung ist noch nicht an einer echten japanischen Mini verifiziert. Die Ansicht bildet nicht sämtliche zusätzlichen Fn-Symbole und AltGr-Aufdrucke aus GG ab. Geräte-IDs und vorhandene Profile bleiben erhalten; es werden keine Regions- oder Firmwareänderungen gesendet. Die Windows-Eingabesprache muss zu den gewünschten Zeichen passen und wird von ApexControl nicht umgestellt.

Das eigene ApexControl-Logo zeigt ein helles A in einer orangefarbenen Hexagon-Keycap. Es erscheint in der App, als Fenster-/Tray-Symbol und als eingebettetes Windows-EXE-Symbol. Die SVG-, PNG- und ICO-Dateien liegen unter `assets/`; die installierte App benötigt dafür keine externen Bilddateien.

## Bedienung

### Profilsicherungen

**Sicherungen** oben öffnet eine Liste der automatisch vor jedem Onboard-Schreibvorgang angelegten Profil-Backups. Angezeigt werden Profilname, ursprünglicher Slot und Alter; der Dateiname ist als Hinweis und in den Details sichtbar. Dateigröße, Format und CRC werden geprüft. Ungültige Sicherungen werden gezählt und übersprungen, nicht gelöscht. Die Liste kann auch ohne angeschlossene Tastatur angesehen werden.

Für die Wiederherstellung eine Sicherung, eine angeschlossene unterstützte Mini und den Zielslot wählen. **Sicherung als Entwurf öffnen** zeigt zunächst den Inhalt im Editor; es sendet weder einen Profil- noch einen Live-Schreibbefehl. Live-Vorschau wird dabei ausgeschaltet. Erst **Auf Tastatur speichern** schreibt den Entwurf in den gewählten Slot. Der bestehende Schreibweg sichert zuvor den Zielzustand, prüft ihn auf zwischenzeitliche Änderungen, liest das Ergebnis zurück und versucht bei einem Fehler die Rücksicherung. Eine neue echte Hardware-Wiederherstellung wurde für die Oberfläche nicht ausgeführt; Auswahl, Entwurf und die erzeugte Schreibanforderung wurden mit simulierten Geräten getestet.

Diese Sicherungen betreffen Tastaturprofile. GitHub-Releases betreffen das Programm; sie ersetzen keine Sicherungen persönlicher Tastatureinstellungen.

### Ungespeicherte Profiländerungen

**Ungespeicherte Änderungen** bezieht sich auf Änderungen seit dem zuletzt geladenen oder ausdrücklich gespeicherten Entwurf. **Nicht auf Tastatur gespeichert** bleibt unabhängig davon sichtbar, wenn der Entwurf vom gewählten Onboard-Zielprofil abweicht. Ein Speichern auf dem PC beendet deshalb die Änderungswarnung, bestätigt aber keinen Onboard-Schreibvorgang. „Live anwenden“ ist kein dauerhaftes Speichern.

Vor Profil-/Gerätewechsel, Geräteaktualisierung, Rückkehr zur Geräteübersicht, Laden einer Sicherung oder vollständigem Beenden fragt die App bei Profiländerungen nach. **Weiter bearbeiten** erhält den Entwurf. **Auf PC speichern und fortfahren** führt nur nach erfolgreichem lokalen Speichern weiter; ein Fehler lässt den Dialog offen. **Verwerfen und fortfahren** verwirft die Änderungen seit dem letzten Lade-/Speicherstand. Das Minimieren in den Infobereich beendet die App nicht und erhält den Entwurf.

**Änderungen zurücksetzen** stellt nach Bestätigung den letzten Lade-/Speicherstand des Entwurfs wieder her, unabhängig vom gewählten Zielslot. Es schaltet Live-Vorschau aus und schreibt nicht auf die Tastatur. Bereits live übertragene Einstellungen bleiben zunächst bestehen; sie lassen sich durch erneutes Live-Anwenden des zurückgesetzten Entwurfs oder Laden eines Onboard-Profils ersetzen. Die Warnung bezieht sich auf das Profil einschließlich zugewiesener Makros, nicht auf noch unzugewiesene Aufnahmen im Makro-Editor. RGB bleibt eine getrennte Live-Funktion.

### Profil-Editor

- Auf der Startseite ein Onboard-Profil wählen und direkt laden oder das Gerät öffnen.
- Im Editor links zwischen fünf Onboard-Profilen und auf dem PC gespeicherten Entwürfen wechseln. Ein Onboard-Profil wird dabei auch auf der Tastatur aktiviert.
- Tasten im Keyboard auswählen; Umschalt + Klick wählt mehrere. Umschalt + linke Maustaste ziehen erzeugt eine Auswahlbox; mit zusätzlichem Strg wird die Auswahl ergänzt. Auslösepunkt, Rapid Trigger und Schutzmodus einstellen.
- Für eine direkte Umbelegung den Reiter **Tastenbelegung** öffnen: oben **B** anklicken, Kategorie **Buchstaben**, Ausgabe **I**, dann **Zuweisen**. Die Ansicht zeigt **B → I** und die Tastatur markiert die geänderte Ausgabe. Mit **Auf Tastatur speichern** wird die Zuordnung aktiviert; **Originaltaste** stellt im Entwurf die normale Belegung wieder her. Die gleichen Kategorien bieten auch F1–F24, Umlaute, Zahlen, Strg/Alt/Shift, Navigation und Ziffernblock. Dafür ist keine Makroaufnahme nötig. Im Makro-Editor führt **Tastenbelegung öffnen** direkt dorthin.
- **Live anwenden** überträgt ausschließlich Auslösepunkt, Rapid Trigger und Schutzmodus in den flüchtigen Gerätespeicher. Die optionale Live-Vorschau überträgt diese Einstellungen nach einer Änderung automatisch.
- **Auf PC speichern** speichert einen vollständigen Profilentwurf lokal. Ein lokales Profil zu öffnen ändert die Tastatur noch nicht.
- **Auf Tastatur speichern** schreibt den Entwurf in den ausgewählten Slot und aktiviert ihn. Tastenbelegungen und Makros werden über diesen Weg übernommen. Vor jedem Schreiben entsteht eine Sicherung; danach wird das ganze Profil zurückgelesen und verglichen. Scheitert der Schreibvorgang, versucht die App, das ursprüngliche Profil wiederherzustellen.
- Der Makro-Editor nimmt einfache Tastaturfolgen mit Pausen auf. Einmalige Wiedergabe beim Drücken, maximal 1000 Ereignisse. Aktuell nur das erste native Makro eines makrofreien Profils; vorhandene Makrodaten werden nicht überschrieben. Keine Mausaktionen, Wiederholungsschleifen oder Modifier-Aufnahme.
- Zusätzlich lassen sich Makros ohne Aufnahme zusammenstellen: Kategorie und Taste wählen, dann **Drücken + Loslassen**, **Nur drücken** oder **Nur loslassen** einfügen. Kategorien: Buchstaben, Umlaute/ß, Zahlen, F1–F24, Steuerung (Strg, Alt, Shift, Windows usw.), Navigation, Sonderzeichen und Ziffernblock. Pausen sind einstellbar; Ereignisse lassen sich entfernen. Beispiel Strg+C: Strg nur drücken → C drücken und loslassen → Strg nur loslassen. Umlaute und Sonderzeichen folgen dem Betriebssystem-Tastaturlayout. Die automatische Aufnahme von Modifiern bleibt eingeschränkt; manuelles Einfügen unterstützt sie.
- **Beleuchtung** bietet **Statisch**, **Atmen**, **Regenbogenwelle** und **Reaktive Welle**, mit Helligkeit und bei Effekten Geschwindigkeit. Es werden ausschließlich die markierten Tasten angesteuert; für die ganze Tastatur **Alle auswählen** verwenden. **Auf Auswahl anwenden** startet die gewählte Farbe beziehungsweise den Effekt. Die zuvor standardmäßig aktive separate Option „Auf alle Tasten anwenden“ wurde entfernt. Statische Farben auf anderen Tasten bleiben erhalten.
- Im RGB-Reiter zeigen die gesamten Tastenkappen die zuletzt bestätigten, von ApexControl gesendeten RGB-Werte einschließlich der Helligkeit. Beschriftungen bleiben mit angepasstem Kontrast lesbar. Actuation-Zahlen und Makro-/Schutzmarkierungen werden dort ausgeblendet. Die Vorschau liest keine LED-Farben aus dem Gerät; unbekannte bzw. native Onboard-Effekte werden neutral dargestellt.
- Animationen sind **Software-Effekte**, keine neu entschlüsselten nativen Onboard-Effekte. Sie nutzen den bereits untersuchten direkten Farbbefehl `0x40`. Der serielle Geräte-Worker erzeugt höchstens ungefähr 30 Bilder pro Sekunde, ohne einen Rückstau von Frames; das tatsächliche Tempo hängt von den USB-Antworten ab. Auch wenn das Fenster im Infobereich verborgen ist, läuft der Worker weiter. Die UI hält nur das jüngste Vorschaubild. Andere Geräteaufträge werden seriell abgearbeitet und können den Effekt vorübergehend pausieren. Es läuft maximal ein Animationseffekt gleichzeitig.
- Reaktive Beleuchtung verwendet unter Windows Raw Input über das vorhandene winit-Zielfenster. Nur solange dieser Modus aktiv ist, wird die Hintergrundzustellung eingeschaltet; beim Stoppen wird die vorherige Registrierung wiederhergestellt. Eingaben anderer Modelle werden anhand der Gerätekennung verworfen. Make-Codes werden direkt Lichtpositionen zugeordnet, nicht in Text umgewandelt oder in Dateien/Netzwerkprotokollen aufgezeichnet. Maximal 64 ausstehende Tastenimpulse und 32 kurzlebige Wellen werden gehalten; Wellen verfallen nach 2,5 Sekunden. Bei mehreren kabelgebundenen Minis können deren Signale denselben Effekt auslösen. Geräte-Remapping und Fn-Schichten können den Ursprung verschieben, da Windows die vom Gerät ausgegebenen Tasten meldet. Fn allein liefert normalerweise keinen eigenen Tastendruck.
- **Onboard-Beleuchtung** stoppt die Software-Effekte und gibt die Steuerung an den gespeicherten Effekt zurück. RGB schreibt keine Onboard-Profile und wird derzeit nicht mit lokalen Profilentwürfen gespeichert. Nach einem Prozessabbruch kann die letzte flüchtige Farbe stehen bleiben; sie ist kein dauerhaft gespeicherter Animationseffekt.
- Die Berechnung der Effekte, Auswahl, Farbvorschau und Rücknahme der Windows-Eingabeanmeldung sind getestet. Am 02.11.2026 bestätigt der Nutzer an seiner Apex Pro Mini: Einzelfarbe auf B, die RGB-Modi einschließlich reaktiver Welle sowie Auswahlbox und anschließendes Einfärben funktionieren. Das ist eine praktische Nutzerbestätigung, keine unabhängige Messung aller LEDs oder eine Prüfung weiterer Firmwarestände. GG muss für aussagekräftige Hardwaretests vollständig beendet sein.

Profile und Sicherungen liegen unter `%LOCALAPPDATA%\ApexControl`. Es werden keine persönlichen GG-Daten mit der App ausgeliefert. Onboard-Profilnamen sind auf 31 UTF-8-Bytes begrenzt. Das dargestellte Layout ist Deutsch ISO.

## Geprüfter Stand

Am angeschlossenen Mini-Gerät mit Firmware 1.19.7:

- Firmware und alle fünf Onboard-Profile nativ mit Rust ausgelesen; Profilprüfsummen gültig.
- Auslösepunkt, Rapid Trigger und Empfindlichkeit gesendet, vom Gerät bestätigt und zurückgestellt.
- Schutzmodus gesendet, bestätigt und zurückgestellt.
- Profil geschrieben und bytegenau zurückgelesen.
- X/Y-Makro samt B-Zuweisung geschrieben, bytegenau zurückgelesen und Originalprofil wiederhergestellt.
- Offline: sechs Live-Pakete und eine vollständige Makro-Profilabbildung stimmen bytegenau mit den tatsächlichen GG-Mitschnitten überein.
- RGB: direkter Farbbefehl vom Gerät mit Status 0 bestätigt; Onboard-Beleuchtung wieder angefordert und unverändertes gespeichertes Profil geprüft.
- Der Nutzer bestätigt beim wiederholten RGB-Test: B leuchtet grün und kehrt danach zur bisherigen Beleuchtung zurück.
- Oberflächenfehler bei der Gerätekarte nach Nutzerfeedback behoben; der Nutzer bestätigt Gerätekarte und Tastatur-Editor. Ein Layouttest mit zwei Geräten bei 1100 und 1320 Punkten Fensterbreite prüft einzeilige Schaltflächen innerhalb der Fenstergrenzen.
- Ergänzende Tests prüfen den Start in der Geräteübersicht ohne USB-Umschaltbefehl, die Wiederaufnahme eines lokalen Profils nach explizitem Öffnen mit frischer Onboard-Sicherungsgrundlage, die Darstellung der manuellen Makroauswahl und das Kompilieren/Lesen eines Makros mit Strg+C, F24 und Ä. Diese neuen Sondertasten sind noch nicht alle physisch am Gerät geprüft.
- Die Desktop-Tests prüfen persistierte Fensteroptionen, korrekt gequotete Autostart-Pfade, die Windows-Registry-Lese-/Schreibfunktionen in einem privaten temporären Testschlüssel und die Schließregeln bei fehlendem Tray-Symbol bzw. laufenden Geräteaufträgen. Die Tests verändern keinen echten Windows-Autostarteintrag.
- Dreizehn Tests bestehen, einschließlich der Hintergrundstart-Reihenfolge nach dem ersten Fensteraufbau. Das Wiederöffnen aus dem Infobereich verwendet jetzt direkt das eigene Win32-Fensterhandle, bevor egui zur nächsten Aktualisierung aufgeweckt wird. Ein nativer Regressionstest prüft an einem eigenen Testfenster das Wiederanzeigen nach Ausblenden und Minimieren ohne Zeichenschleife sowie das sichere Ablehnen eines bereits zerstörten Handles. Ein echter Windows-Abmelde-/Anmeldevorgang wurde nicht durchgeführt.
- Nach Einspielen der Korrektur bestätigt der Nutzer: Sowohl Doppelklick auf das Tray-Symbol als auch Rechtsklick → „ApexControl öffnen“ zeigen das zuvor ausgeblendete Fenster wieder an.
- Ein zusätzlicher UI-Test wählt I als Ausgabe für B, erhält diese Auswahl über mehrere Bildaufbauten und klickt **Zuweisen**. Er prüft die resultierende B-Belegung, eine unveränderte I-Taste, die Profilprüfsumme und dass erst ein explizites Speichern einen Geräteauftrag auslösen darf. Insgesamt bestehen vierzehn Tests.
- Weitere Prüfungen rendern die englische Oberfläche, prüfen Platzhalter in allen Übersetzungen und stellen sicher, dass eigene Profilnamen unverändert bleiben. Die gespeicherte Sprachwahl wird mit den Fensteroptionen zurückgelesen. Insgesamt bestehen siebzehn Tests.
- Die Ersteinrichtung wird auf englischen Fragetext, Fokus, fehlenden Pfeil und Ausblenden nach abgeschlossener Auswahl geprüft. Der UI-Test klickt beide Sprachschaltflächen und wählt anschließend US-ANSI im entsprechend lokalisierten Dropdown. Der Speicher-/Neustart-Test prüft die gemeinsamen Angaben und Abschlusskennzeichen einschließlich Speicherfehlern. Layouttests prüfen 61/62 eindeutige Tasten, Zeilenbreiten, unveränderte HID-IDs und passende deutsche, US- und spanische Beschriftungen unabhängig von der Menüsprache. Insgesamt bestehen zweiundzwanzig Tests.

Die Bestätigungen prüfen das Protokoll und die gespeicherten Daten. Physische Auslösetiefen und die tatsächliche Makroausgabe wurden noch nicht unabhängig gemessen. Weitere Firmwarestände, Linux und andere Apex-Modelle sind nicht getestet. Keine Firmwareupdates, keine eigenen Gerätetreiber.

Die Computersteuerung hat keinen Zugriff auf das neue ApexControl-Fenster. Die visuelle Kontrolle erfolgte daher über Nutzerfeedback; das ist keine vollständige automatisierte Bedienprüfung.

## Entwicklung

`build.ps1` verwendet die lokal vorbereitete Rust-GNU-Toolchain unter `.tools/`. Sie wird nicht in die Veröffentlichung aufgenommen. Auf einer eigenen Entwicklungsmaschine Rust und einen passenden Windows-Linker installieren und `cargo build --release` verwenden. Abhängigkeiten sind in `Cargo.lock` festgehalten.

Der Windows-Build bettet mit `winresource` das ICO-Symbol ein; dafür wird bei GNU `windres` beziehungsweise bei MSVC der Windows-Ressourcencompiler verwendet. `tools/prepare_brand.py` erzeugt mit Pillow die mitgelieferten Icon-Rasterdaten aus den im SVG dokumentierten Formen. Pillow wird nur zum Neugenerieren der Logo-Dateien benötigt, nicht zum Bauen oder Starten der App. Deutsche Originaltexte und die englische Übersetzung unter `locales/en.json` bilden die Sprachdateien.

```powershell
.\build.ps1
.\build.ps1 -Test
.\target\debug\apexcontrol.exe --offline-check ..\apex-usb-research --report .\local-data\offline-check.json
.\target\debug\apexcontrol.exe --hardware-check --report .\local-data\hardware-read.json
.\build.ps1 -Release
```

`--offline-check` benötigt die privaten lokalen Mitschnitte und Sicherungen, die nicht veröffentlicht werden. `--hardware-check` liest nur. `exercise-windows.ps1` ist ein Entwicklungswerkzeug: es benötigt Administratorrechte zum vorübergehenden Beenden von GG und führt echte, rückgesicherte Profil-Schreibtests aus. Nicht als allgemeines Nutzerwerkzeug gedacht.

## Technischer Aufbau

`protocol.rs` erzeugt und prüft Mini-Profile, CRCs, Live-Reports und einfache Makros. `backend.rs` kommuniziert über Windows HID und führt die Aufträge in einem Hintergrundthread aus. `ui.rs` enthält die native egui-Oberfläche. Unbekannte Profilfelder, Fn-Belegungen und zusätzliche Profildaten werden erhalten.

Die tatsächlichen lokalen GG-/USB-Mitschnitte sind die Referenz für das Mini-Protokoll. Als Vergleich dienten die öffentlichen Protokollbeschreibungen von [Rikearon/apex-control](https://github.com/Rikearon/apex-control) und [steelseriesgg-rs](https://github.com/Ven0m0/steelseriesgg-rs); deren Geräteunterstützung ist kein Nachweis für die Mini. Es wurde kein SteelSeries-Programmcode übernommen. Die numerische Auslösepunkt-Tabelle stammt aus der installierten GG-Version.

Dies ist ein unabhängiges Projekt und kein Produkt von SteelSeries.

## Lizenz

ApexControl einschließlich des eigenen Logo-Artworks wird unter **GPL-3.0-or-later** veröffentlicht: GNU General Public License, Version 3 oder nach eigener Wahl eine spätere Version. Der vollständige Lizenztext steht in [LICENSE](LICENSE). Abhängigkeiten behalten ihre jeweiligen Lizenzen; siehe [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Forks und Erweiterungen für weitere Geräte sind ausdrücklich willkommen. Wer eine auf ApexControl aufbauende Version weitergibt, muss die GPL-Bedingungen einhalten und den Empfängern den entsprechenden vollständigen Quellcode einschließlich erforderlicher Build-Skripte unter der GPL zugänglich machen. Rein private Änderungen müssen nicht veröffentlicht werden. Bei einer öffentlichen Binärveröffentlichung muss auch die Bereitstellung des zugehörigen Quellcodes nach GPL geregelt sein; die Lizenzdatei allein reicht dafür nicht aus.
