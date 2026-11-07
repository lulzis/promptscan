# Sicherheit

## Unterstützte Fassungen

| Fassung | Unterstützt |
| --- | --- |
| 0.1.x (Testfassung) | ja |

## Lücke melden

Sicherheitslücken bitte nicht als öffentliches Issue melden, sondern vertraulich über
[GitHub Security Advisories](https://github.com/lulzis/promptscan/security/advisories/new)
(«Report a vulnerability»).

Hilfreich sind: betroffene Fassung, Schritte zum Nachstellen und, falls bekannt, die
Auswirkung. Bitte keine echten vertraulichen Daten in die Meldung kopieren, sondern ein
erfundenes Beispiel, das denselben Fehler zeigt.

Eingang wird innert 7 Tagen bestätigt. Eine bestätigte Lücke wird behoben und nach der
Behebung mit einem Advisory veröffentlicht.

## Was als Lücke gilt

- promptscan sendet Daten, startet ein Programm oder schreibt eine Datei ohne Befehl.
- Ein geprüfter Text gelangt in eine Datei, ein Protokoll oder die Konfiguration.
- Eine Eingabe bringt promptscan zum Absturz oder hält es unbegrenzt auf.
- Die weitergegebene Programmdatei stimmt nicht mit der veröffentlichten Prüfsumme überein.

Ein nicht erkanntes Personendatum ist eine Lücke der Erkennung, keine Sicherheitslücke; sie
gehört in ein gewöhnliches Issue, ebenfalls ohne echte Daten.

## Security policy (English)

Please report vulnerabilities privately via
[GitHub Security Advisories](https://github.com/lulzis/promptscan/security/advisories/new),
not as a public issue, and use made-up example data only. Reports are acknowledged within
7 days.
