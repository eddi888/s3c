# s3c - S3 Commander

Ein Midnight Commander-inspiriertes Terminal User Interface (TUI) für S3-Dateiverwaltung mit AWS-Profil-Unterstützung und Dual-Panel Design.

## Features

### 🎨 Benutzeroberfläche
- 📊 **Dual-Panel Mode** - Zwei Panels nebeneinander (S3 ↔ Local, S3 ↔ S3)
- ⌨️ **MC-Style Footer Menu** - F1-F10 Funktionstasten mit kontextabhängigen Funktionen
- 📋 **Columnar Display** - Name, Size, Modified (wie Midnight Commander)
- 🎨 **Cyan-Theme** - Authentische MC-Farbgebung
- 🔄 **Tab-Navigation** - Wechsel zwischen linkem und rechtem Panel

### 🔐 AWS Integration
- 👤 **AWS Profile Management** - Nutzt Credentials aus `~/.aws/credentials`
- 🪣 **Bucket Zuordnung** - Profile können individuell mit S3 Buckets verknüpft werden
- 🔗 **Role Chaining** - Unterstützung für mehrfaches Role Assumption (Role über Role)
- 🌍 **Multi-Region Support** - Konfigurierbare AWS Regions pro Bucket
- 🔧 **Setup Scripts** - Automatische Ausführung von Authentifizierungs-Scripts (z.B. `aws-vault`, `aws sso`)

### 📂 Dateiverwaltung
- 🗂️ **S3 Browser** - Navigation durch S3 Buckets und Objekte
- 💻 **Local Filesystem** - Lokales Dateisystem durchsuchen
- 👁️ **File Preview** - Vorschau für S3 und lokale Dateien (max 1MB)
- ⬇️ **Download** - S3 → Local mit Pfad-Eingabe
- ⬆️ **Upload** - Local → S3 mit Ziel-Pfad-Eingabe
- 📁 **S3 Folder Creation** - Erstellen von S3 "Ordnern" (Prefix-Marker)
- 🗑️ **Delete** - Löschen von S3-Objekten und lokalen Dateien
- 🔙 **Back Navigation** - ".." Einträge für intuitive Navigation

### 🛡️ Robustheit
- ⚠️ **Error Handling** - Graceful handling von NoSuchBucket, AccessDenied, Permission denied
- 🔒 **Permission Checks** - Keine Abstürze bei fehlenden Rechten
- ℹ️ **User-friendly Messages** - Klare Fehlermeldungen statt Crashes

## Installation

```bash
# Repository klonen
git clone <repository-url>
cd s3c

# Build und Run
cargo build --release
cargo run
```

## Voraussetzungen

- Rust 1.70 oder höher
- AWS Credentials konfiguriert in `~/.aws/credentials`
- Gültige AWS Profile mit S3 Zugriffsrechten

## AWS Konfiguration

Stelle sicher, dass deine AWS Credentials korrekt konfiguriert sind:

```bash
# ~/.aws/credentials
[profile1]
aws_access_key_id = YOUR_ACCESS_KEY
aws_secret_access_key = YOUR_SECRET_KEY

[profile2]
aws_access_key_id = YOUR_ACCESS_KEY
aws_secret_access_key = YOUR_SECRET_KEY
```

## Verwendung

### 1. Anwendung starten

```bash
cargo run --release
```

### 2. Dual-Panel Navigation

Die Anwendung startet mit zwei Panels:
- **Linkes Panel:** AWS Profile
- **Rechtes Panel:** Lokales Dateisystem (Home-Verzeichnis)

**Grundlegende Navigation:**
- **Tab** - Zwischen Panels wechseln
- **↑/↓** - In Listen navigieren
- **PgUp/PgDn** - Seitenweise scrollen
- **Enter** - Auswahl bestätigen / Ordner öffnen
- **F10** / **q** - Anwendung beenden

### 3. Profile und Setup Scripts

**Profil auswählen:**
1. Navigiere im **ProfileList** zu einem Profil (👤)
2. Drücke **Enter**
3. Falls ein Setup-Script konfiguriert ist, wird es **automatisch ausgeführt**
4. Nach erfolgreicher Ausführung erscheint die **BucketList**

**Setup-Script konfigurieren:**
- Drücke **F4** oder **P** auf einem Profil
- Gib den Script-Pfad oder Befehl ein, z.B.:
  ```bash
  aws-vault exec myprofile -- true
  ```
- Das Script wird **vor der Bucket-Anzeige** ausgeführt
- **Interaktive Eingaben** (MFA-Codes, etc.) werden unterstützt

### 4. Bucket Management

**Neue Bucket-Konfiguration erstellen:**
- Im **BucketList**, drücke **F2** oder **B**
- Eingaben:
  - **Bucket Name**
  - **Region** (z.B. eu-west-1)
  - **Beschreibung** (optional)
  - **Role Chain** (optional, mehrere Roles möglich)

**Bucket-Konfiguration bearbeiten:**
- Im **BucketList**, drücke **F4** oder **E** auf einem Bucket
- Zum Löschen: **D** auf einem Bucket

### 5. S3 und Lokales Dateisystem

**S3 Browser:**
- **Enter** auf Bucket → S3-Objekte werden geladen
- **F3/V** - Datei-Vorschau (max 1MB)
- **F5/C** - Download zu anderem Panel
- **F7/M** - Neuen S3-Ordner erstellen
- **F8/Del** - Objekt löschen
- **..** - Zurück zur Bucket-Liste

**Local Filesystem:**
- Navigation wie S3 Browser
- **F3/V** - Lokale Datei anzeigen
- **F5/C** - Upload zu S3 Panel
- **F8/Del** - Lokale Datei löschen
- **..** - Zum Parent-Verzeichnis

## Keyboard Shortcuts

### MC-Style Function Keys (Kontextabhängig)

```
01Help  02Create  03View  04Edit  05Copy  06Move  07Mkdir  08Delete  09Menu  10Exit
```

| Taste | Funktion | Kontext | Alternative |
|-------|----------|---------|-------------|
| **F1** | Help | Alle | `?` |
| **F2** | Create | BucketList | `B` |
| **F3** | View | S3/Local | `V` |
| **F4** | Edit | ProfileList/BucketList | `P`, `E` |
| **F5** | Copy | S3/Local | `C` |
| **F6** | Move | - | - |
| **F7** | Mkdir | S3Browser | `M` |
| **F8** | Delete | S3/Local/BucketList | `Del`, `D` |
| **F9** | Menu | - | - |
| **F10** | Exit | Alle | `q` |

### Navigation
- **Tab** - Zwischen Panels wechseln
- **↑/↓** - Hoch/Runter in Listen
- **PgUp/PgDn** - Seitenweise scrollen (basierend auf Panel-Höhe)
- **Enter** - Auswahl bestätigen / Ordner öffnen
- **Esc** - Zurück / Abbrechen
- **F** - Switch to local Filesystem (von ProfileList)

### Input-Dialoge
- **Enter** - Eingabe bestätigen
- **Backspace** - Zeichen löschen
- **Esc** - Abbrechen

## Konfiguration

Die Anwendung speichert Profil-Bucket-Zuordnungen in:
```
~/.config/s3c/config.json
```

Format:
```json
{
  "profiles": [
    {
      "name": "profile1",
      "description": "Production Environment",
      "setup_script": "aws-vault exec profile1 -- true",
      "buckets": [
        {
          "name": "my-bucket-1",
          "region": "eu-west-1",
          "description": "Main storage bucket"
        },
        {
          "name": "my-bucket-2",
          "region": "us-east-1",
          "description": "Cross-account bucket",
          "role_chain": [
            "arn:aws:iam::123456789012:role/FirstRole",
            "arn:aws:iam::987654321098:role/SecondRole"
          ]
        }
      ]
    }
  ]
}
```

### Konfigurationsfelder

**Profile:**
- `name` - AWS Profil-Name (muss in `~/.aws/credentials` existieren)
- `description` - Optionale Beschreibung (wird in UI angezeigt)
- `setup_script` - Optionales Script/Befehl, der vor Bucket-Anzeige ausgeführt wird
- `buckets` - Liste der konfigurierten Buckets

**Buckets:**
- `name` - S3 Bucket-Name
- `region` - AWS Region (z.B. "eu-west-1", "us-east-1")
- `description` - Optionale Beschreibung
- `role_chain` - Optionale Liste von Role ARNs für Role Chaining

## Setup Scripts

### Was sind Setup Scripts?

Setup Scripts sind Shell-Befehle oder Scripts, die automatisch ausgeführt werden, bevor die Bucket-Liste eines Profils angezeigt wird. Dies ist besonders nützlich für:
- **Authentifizierung** mit Tools wie `aws-vault` oder `aws sso`
- **MFA-Token Eingabe** vor S3-Zugriff
- **Credential-Refresh** für zeitlich begrenzte Tokens
- **Custom AWS Configuration** pro Profil

### Wie funktioniert es?

1. **Profil auswählen** - Enter auf ein Profil in der ProfileList
2. **TUI wird suspendiert** - Normales Terminal erscheint
3. **Script läuft interaktiv** - Du siehst die Ausgabe und kannst Eingaben machen
4. **TUI kommt zurück** - Nach erfolgreicher Ausführung
5. **Bucket-Liste wird angezeigt** - Mit den frischen Credentials

### Beispiele

**Mit aws-vault (MFA-Authentifizierung):**
```bash
aws-vault exec production -- true
```

**Mit AWS SSO:**
```bash
aws sso login --profile myprofile
```

**Multi-Step Script:**
```bash
~/scripts/s3-auth.sh && echo "Authentication successful"
```

### Interaktive Eingaben

Setup Scripts unterstützen **vollständig interaktive Eingaben**:
- ✅ MFA-Code Eingabe
- ✅ Passwort-Prompts
- ✅ Beliebige Benutzer-Interaktionen
- ✅ Farbige Terminal-Ausgabe

Die TUI wird temporär beendet und das normale Terminal übernimmt.

## AWS Role Chaining

### Was ist Role Chaining?

Role Chaining ermöglicht es, mehrere IAM Roles nacheinander anzunehmen (Role Assumption). Dies ist nützlich wenn:
- Cross-Account Zugriff über mehrere AWS Accounts erforderlich ist
- Sicherheitsrichtlinien mehrere Role-Hops erfordern
- Komplexe AWS-Organisationsstrukturen existieren

### Wie funktioniert es?

1. **Schritt 1:** Anmeldung mit AWS Profil aus `~/.aws/credentials`
2. **Schritt 2:** Erste Role wird mit Profil-Credentials angenommen
3. **Schritt 3:** Zweite Role wird mit Credentials aus Schritt 2 angenommen
4. **Schritt N:** Weitere Roles werden nacheinander angenommen
5. **Zugriff:** Finale Credentials werden für S3-Zugriff verwendet

### Beispiel: Cross-Account S3 Zugriff

```bash
# Szenario: Zugriff auf Bucket in Account B über Account A
# Profil "production" → CrossAccountRole (Account A) → S3AccessRole (Account B) → Bucket
```

Konfiguration in s3c:
```json
{
  "profiles": [
    {
      "name": "production",
      "buckets": [
        {
          "name": "company-data-archive",
          "role_chain": [
            "arn:aws:iam::111111111111:role/CrossAccountRole",
            "arn:aws:iam::222222222222:role/S3AccessRole"
          ]
        }
      ]
    }
  ]
}
```

### In der UI:

- **Bucket Selection:** Zeigt `bucket-name (Roles: 2)` an
- **Bucket Management:** Zeigt Role-Kette als `Role1 → Role2` an
- **Bei Fehlern:** Zeigt genau an, welche Role in der Kette fehlgeschlagen ist

### Hinweise:

- Jede Role muss die Berechtigung haben, die nächste Role anzunehmen
- Die letzte Role muss S3-Zugriffsrechte haben
- Session-Tokens haben begrenzte Gültigkeit (typisch 1 Stunde)
- Bei Fehlern wird angezeigt: "Failed to assume role X (step Y of Z)"

## Architektur

### Module

- **`src/main.rs`** - Event-Loop und Input-Handler
- **`src/app.rs`** - Anwendungslogik und State-Management
- **`src/config.rs`** - AWS Profile und Bucket-Konfiguration
- **`src/s3_ops.rs`** - S3 API-Operationen
- **`src/ui.rs`** - TUI-Komponenten und Rendering

### Technologie-Stack

- **Ratatui** - TUI Framework
- **Crossterm** - Terminal Manipulation
- **AWS SDK für Rust** - S3 Operationen
- **Tokio** - Async Runtime
- **Serde** - JSON Serialisierung

## UI-Design

### Midnight Commander Inspiration

s3c ist von Midnight Commander (MC) inspiriert und übernimmt dessen bewährtes Design:

**Dual-Panel Layout:**
- Zwei Panels nebeneinander für effiziente Datei-Operationen
- Tab-Navigation zwischen Panels
- Konsistente Farbgebung (Cyan/Yellow)

**Function-Key Menu:**
```
01Help  02Create  03View  04Edit  05Copy  06Move  07Mkdir  08Delete  09Menu  10Exit
```
- Kontextabhängige Funktionen (ändern sich je nach Panel-Typ)
- Zahlen mit schwarzem Hintergrund, Labels mit Cyan
- Gleichmäßige Verteilung über volle Terminalbreite

**Columnar Display:**
- Name, Size, Modified in fester Spaltenbreite
- Truncation bei langen Namen
- Icons für Dateitypen (📄, 📁, 👤, etc.)

## Fehlerbehebung

### Keine Profile gefunden
Stelle sicher, dass `~/.aws/credentials` existiert und gültige Profile enthält.

### Setup Script Fehler
- **"Setup script failed"**: Script ist mit Exit-Code != 0 beendet
- Prüfe Script-Berechtigungen und Pfad
- Teste Script manuell im Terminal: `sh -c "dein-script"`

### AWS Fehler (werden graceful behandelt)

**NoSuchBucket:**
- Fehlermeldung: "Bucket 'xyz' does not exist or is in wrong region"
- Keine Crashes mehr, nur freundliche Meldung

**AccessDenied:**
- Fehlermeldung: "Access denied to bucket 'xyz': Check permissions"
- Bei S3-Operationen und Role-Assumption

**Permission denied (lokales Dateisystem):**
- Fehlermeldung: "Permission denied: Cannot access '/path'"
- Bei Read-, Write- oder Delete-Operationen

### Role Chaining Fehler
- **"Failed to assume role X (step Y of Z)"**: Role kann nicht angenommen werden
- Prüfe Trust Policy und Berechtigungen
- Jede Role muss der vorherigen Role vertrauen

### Build-Fehler
```bash
# Sicherstellen, dass alle Dependencies aktuell sind
cargo clean
cargo build --release
```

## Lizenz

Siehe LICENSE Datei.
