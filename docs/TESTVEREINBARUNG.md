# Testvereinbarung promptscan

> **Entwurf.** Kein geprüfter Vertragstext und keine Rechtsauskunft. Eckige Klammern sind vor
> Gebrauch auszufüllen oder zu streichen.
>
> Belege zu Ziffer 2: Dass promptscan nichts sendet, keine anderen Programme startet und keine
> Dateien schreibt, prüft `tests/offline.rs` am Quelltext und an der Programmdatei (SPEC 6).
> Zum Verlauf der Shell Microsoft, about_PSReadLine (Windows PowerShell 5.1, Stand 16.06.2026):
> «PSReadLine maintains a history file containing all the commands and data you've entered
> from the command line.»

Zwischen [Rechteinhaber] (Anbieter) und [Name der Testperson] (Testperson).

**1. Zweck.** Die Testperson prüft im Nutzertest (SPEC 7.6) eine Testfassung von promptscan:
Sie prüft eigene, erfundene Prompts und spricht dabei laut mit. Erfasst wird, welche
Fehlalarme stören und welche Erklärungen verständlich sind. [Ort, Dauer, Aufzeichnung ja/nein.]

**2. Nur erfundene Prompts.** Die Testperson gibt nur Prompts ein, die sie für den Test
erfindet. Sie gibt keine echten Personendaten ein, nichts aus Mandaten, Dossiers oder
Personalakten und nichts, was einem Berufs- oder Amtsgeheimnis untersteht. promptscan prüft
lokal und sendet nichts. Dateien schreibt es nur auf Befehl: `promptscan config --role …`
speichert die Rolle (`--lang …` die Sprache) in seiner Konfigurationsdatei, `--output <datei>`
den Bericht samt dem geprüften Text in die genannte Datei. Wird der Text in der Befehlszeile
übergeben (mit `-t` oder etwa als `"…" | promptscan`), steht er aber im Verlauf der Shell.
Darum den Text einfügen (`promptscan` ohne Argument) oder als Datei übergeben.

**3. Keine Weitergabe.** Die Testfassung (Programmdatei und Unterlagen) bleibt bei der
Testperson. Sie gibt sie nicht weiter, veröffentlicht sie nicht und macht sie niemandem
zugänglich. Nach dem Test löscht sie die Testfassung [bis spätestens …]. Es gilt LICENSE.md
der Testfassung (`promptscan licenses`).

**4. Kein Verlass auf die Ergebnisse.** Die Testfassung ist unfertig. Ihre Meldungen sind keine
Rechtsauskunft und keine Prüfung, ob ein Prompt weitergegeben werden darf; promptscan meldet
Datenkategorien, nie Rechtsverletzungen (SPEC 2, Grundsatz 4). Die Testperson stützt keine
Entscheidung auf die Testfassung.

**5. Rückmeldung.** Die Testperson meldet Fehlalarme, übersehene Stellen und unklare
Erklärungen [Weg, Frist]. Der Anbieter darf die Rückmeldungen ohne Namen der Testperson zur
Verbesserung von promptscan verwenden. [Entgelt: keines / …]

**6. Haftung.** Der Anbieter haftet für Schäden aus dem Test nur, soweit ein Ausschluss nach
Art. 100 OR nicht zulässig ist. Art. 100 OR (SR 220, Fassung vom 01.10.2026, Fedlex):

> 1 Eine zum voraus getroffene Verabredung, wonach die Haftung für rechtswidrige Absicht oder
> grobe Fahrlässigkeit ausgeschlossen sein würde, ist nichtig.
>
> 2 Auch ein zum voraus erklärter Verzicht auf Haftung für leichtes Verschulden kann nach
> Ermessen des Richters als nichtig betrachtet werden, wenn der Verzichtende zur Zeit seiner
> Erklärung im Dienst des anderen Teiles stand, oder wenn die Verantwortlichkeit aus dem
> Betriebe eines obrigkeitlich konzessionierten Gewerbes folgt.

Die Haftung für Absicht und grobe Fahrlässigkeit bleibt also bestehen. [Zu prüfen: ob eine
Testperson im Dienst des Anbieters steht (Abs. 2).]

**7. Dauer.** Der Test dauert [von … bis …]. Beide Seiten können ihn jederzeit beenden.
Ziffern 3 und 4 gelten danach weiter.

**8. Recht.** Es gilt Schweizer Recht. [Gerichtsstand.]

Ort, Datum: ………………………………

Anbieter: ………………………………  Testperson: ………………………………
