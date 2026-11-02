# Änderungsübersicht

## 02.11.2026 — Entwicklungsstand 0.1.0

### Neu

- Updates-Bereich mit installierter Version, lokalen Änderungsnotizen und Link zur geplanten GitHub-Releases-Seite. Automatische Updateprüfung und Installation sind noch nicht eingerichtet.
- Französisch AZERTY ISO, Nordisch ISO und Japanisch JIS ergänzt; Deutsch ISO, USA ANSI, UK ISO und Spanisch ISO bleiben verfügbar. Haupt- und Shift-Beschriftungen wurden anhand der GG-Bilder verfeinert; JIS hat eine eigene Geometrie mit 65 Tasten.
- Grafische Profilsicherungen mit Profilname, ursprünglichem Slot, Alter sowie Format- und Prüfsummenprüfung. Eine Sicherung wird zunächst als Entwurf geöffnet; erst „Auf Tastatur speichern“ schreibt sie in den gewählten Zielslot.
- Warnung bei ungespeicherten Profiländerungen vor Profil-/Gerätewechsel, Gerätesuche, Verlassen des Editors oder vollständigem Beenden. Auswahl: auf PC speichern und fortfahren, verwerfen oder weiter bearbeiten. Bei Speicherfehlern bleibt der Dialog offen.
- „Änderungen zurücksetzen“ stellt den zuletzt geladenen/gespeicherten Entwurf wieder her. Lokales Speichern und Onboard-Speichern werden getrennt angezeigt. Zurücksetzen nimmt bereits live angewendete Einstellungen nicht automatisch zurück.
- RGB-Modi Atmen, Regenbogenwelle und reaktive Welle mit Helligkeit und Geschwindigkeit. Es sind Software-Effekte: ApexControl muss weiterlaufen, auch im Infobereich. Die Effekte werden nicht onboard gespeichert.
- Auswahlbox mit Shift + linker Maustaste ziehen. Strg + Shift ergänzt die Auswahl; Shift + Klick wählt weiterhin einzelne zusätzliche Tasten.

### Behoben

- RGB gilt nur für die ausgewählten Tasten. Die standardmäßig aktive Übersteuerung „Auf alle Tasten anwenden“ wurde entfernt; für die ganze Tastatur dient „Alle auswählen“.
- Die RGB-Vorschau färbt die gesamte Taste mit der zuletzt bestätigten Farbe ein und erhält die lesbare Beschriftung. Betätigungswerte und Makromarkierungen werden im RGB-Reiter ausgeblendet.
- Erneute manuelle Starts öffnen das vorhandene Fenster statt weiterer Hintergrundinstanzen. Gleichzeitig gestartete Kopien werden pro Windows-Benutzer und Sitzung abgefangen; doppelte Autostarts bleiben still.
- Der Windows-Autostarteintrag bleibt nach dem Deaktivieren im Task-Manager sichtbar. Die App übernimmt Änderungen von dort und ändert die Startoption nur bei ausdrücklichem Betätigen.
- Die USB-Anzeige zeigt den Stand der letzten Gerätesuche statt dauerhaft grün. Nach dem Umstecken „Geräte aktualisieren“ verwenden.
- Verschärfte Prüfungen für USB-Antwortlängen, Befehlsgrößen, Geräteschnittstellen und JSON-Profile. Begrenzte Dateizugriffe, atomisches Speichern und Schutz vor überlangen Makroaufnahmen innerhalb eines Bildaufbaus.

### Geprüft und noch offen

- 48 automatisierte Tests bestehen. Der Praxistest an der Apex Pro Mini bestätigt Einzelfarben, RGB-Modi einschließlich reaktiver Welle und Auswahlbox mit anschließendem Einfärben.
- Reaktive Beleuchtung nutzt kurzlebige Windows-Tastensignale der kabelgebundenen Mini ohne Text-/Dateiaufzeichnung. Geräte-Umbelegungen und Fn-Schichten können den Ursprung verschieben.
- Andere Tastaturmodelle, weitere Firmwarestände und der Linux-Port sind noch nicht geprüft. Manche zusätzlichen Fn-/AltGr-Aufdrucke fehlen. SECURITY.md beschreibt die gezielte Sicherheitsprüfung und offene Wartungshinweise bei Abhängigkeiten.
