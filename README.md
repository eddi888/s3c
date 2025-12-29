# s3c - S3 Commander

Ein Midnight Commander-inspiriertes Terminal User Interface (TUI) für S3-Dateiverwaltung mit AWS-Profil-Unterstützung und Dual-Panel Design.

## Features

### 🎨 Benutzeroberfläche
- 📊 **Dual-Panel Mode** - Zwei Panels nebeneinander (S3 ↔ Local, S3 ↔ S3)
- ⌨️ **MC-Style Footer Menu** - F1-F10 Funktionstasten mit kontextabhängigen Funktionen
- 📋 **Columnar Display** - Name, Size, Modified (wie Midnight Commander)
- 🎨 **Cyan-Theme** - Authentische MC-Farbgebung
- 🔄 **Tab-Navigation** - Wechsel zwischen linkem und rechtem Panel

### 🔐 AWS & S3-kompatible Services
- 👤 **AWS Profile Management** - Nutzt Credentials aus `~/.aws/credentials`
- 🪣 **Bucket Zuordnung** - Profile können individuell mit S3 Buckets verknüpft werden
- 🔗 **Role Chaining** - Unterstützung für mehrfaches Role Assumption (Role über Role)
- 🌍 **Multi-Region Support** - Konfigurierbare AWS Regions pro Bucket
- 🔧 **Setup Scripts** - Automatische Ausführung von Authentifizierungs-Scripts (z.B. `aws-vault`, `aws sso`)
- 🌐 **S3-kompatible Services** - Unterstützung für Hetzner, Minio, DigitalOcean, Wasabi, Ceph
  - Custom Endpoint URLs konfigurierbar
  - Path-Style URLs für Minio/Ceph

### 🔀 Navigation
- 🎯 **Modus-Übersicht** - Zentrale Auswahl zwischen S3 Storage und Local Filesystem
- 💾 **Windows Laufwerksübersicht** - Automatische Erkennung und Navigation zwischen Laufwerken (C:\, D:\, etc.)
- 🔙 **Intuitive ".." Navigation** - Von überall zurück zur Modus-Auswahl

### 📂 Dateiverwaltung
- 🗂️ **S3 Browser** - Navigation durch S3 Buckets und Objekte
- 💻 **Local Filesystem** - Lokales Dateisystem durchsuchen
- 👁️ **File Preview** - Vorschau für S3 und lokale Dateien
  - Automatischer Zeilenumbruch für lange Zeilen (z.B. einzeilige JSON-Dateien)
  - Visuelles Scrolling inkl. umgebrochener Zeilen
  - END-Taste springt zum visuellen Ende der Datei
  - Lazy Loading für große Dateien (100KB Chunks)
  - Forward/Backward Modus für effiziente Navigation
- ⬇️ **Download** - S3 → Local mit Pfad-Eingabe
- ⬆️ **Upload** - Local → S3 mit Ziel-Pfad-Eingabe
  - Hintergrund-Transfers mit Fortschrittsanzeige
  - Transfer-Abbruch mit 'x'-Taste möglich
- 📁 **S3 Folder Creation** - Erstellen von S3 "Ordnern" (Prefix-Marker)
- ✏️ **Rename** - Umbenennen von Dateien und Ordnern (S3/Local)
- 🔍 **Filter** - Filterung nach Namen in allen Listen
- 📊 **Sort** - Sortierung nach Name, Size oder Date (auf-/absteigend)
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

### AWS CLI installieren

Falls noch nicht installiert:

```bash
# macOS
brew install awscli

# Linux
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install

# Windows
# Download und installiere von: https://aws.amazon.com/cli/
```

### AWS Profil anlegen

**1. Credentials konfigurieren:**

```bash
# Interaktiv (empfohlen)
aws configure --profile myprofile
# Eingaben:
# - AWS Access Key ID
# - AWS Secret Access Key  
# - Default region (z.B. eu-west-1)
# - Default output format (json)
```

**2. Manuelle Konfiguration:**

Erstelle/bearbeite die Dateien:

```bash
# ~/.aws/credentials
[myprofile]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

[production]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

```bash
# ~/.aws/config
[profile myprofile]
region = eu-west-1
output = json

[profile production]
region = us-east-1
output = json
```

**3. Identität überprüfen:**

```bash
# Identität mit spezifischem Profil abrufen
aws sts get-caller-identity --profile myprofile

# Ausgabe:
# {
#     "UserId": "AIDAI...",
#     "Account": "123456789012",
#     "Arn": "arn:aws:iam::123456789012:user/myuser"
# }

# Alle verfügbaren Profile anzeigen
aws configure list-profiles
```

### Mehrere Profile verwalten

```bash
# Profil "development" anlegen
aws configure --profile development

# Profil "staging" anlegen
aws configure --profile staging

# Profil "production" anlegen
aws configure --profile production
```

**Profile in s3c verwenden:**

s3c liest automatisch alle Profile aus `~/.aws/credentials` und zeigt sie beim Start an.

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
- Drücke **F3** auf einem Profil (Edit)
- Gib den Script-Pfad oder Befehl ein, z.B.:
  ```bash
  aws-vault exec myprofile -- true
  ```
- Das Script wird **vor der Bucket-Anzeige** ausgeführt
- **Interaktive Eingaben** (MFA-Codes, etc.) werden unterstützt

### 4. Bucket Management

**Neue Bucket-Konfiguration erstellen:**
- Im **BucketList**, drücke **F7** (Create)
- Eingaben:
  - **Bucket Name**
  - **Region** (z.B. eu-west-1)
  - **Beschreibung** (optional)
  - **Role Chain** (optional, mehrere Roles möglich)

**Bucket-Konfiguration bearbeiten:**
- Im **BucketList**, drücke **F3** (Edit) auf einem Bucket
- Zum Löschen: **F8** (Delete) auf einem Bucket

### 5. S3 und Lokales Dateisystem

**S3 Browser:**
- **Enter** auf Bucket → S3-Objekte werden geladen
- **F2** - Sortierung ändern
- **F3** - Datei-Vorschau
- **F4** - Filter nach Namen
- **F5** - Download zu anderem Panel
- **F6** - Datei/Ordner umbenennen
- **F7** - Neuen S3-Ordner erstellen
- **F8** - Objekt löschen
- **..** - Zurück zur Bucket-Liste

**Local Filesystem:**
- Navigation wie S3 Browser
- **F2** - Sortierung ändern
- **F3** - Lokale Datei anzeigen
- **F4** - Filter nach Namen
- **F5** - Upload zu S3 Panel
- **F6** - Datei/Ordner umbenennen
- **F8** - Lokale Datei löschen
- **..** - Zum Parent-Verzeichnis

**File Preview (F3):**
- **↑/↓** - Zeile für Zeile scrollen (inkl. umgebrochene Zeilen)
- **PgUp/PgDn** - Seitenweise scrollen
- **Home** - Zum Anfang der Datei springen (lädt Head bei Bedarf)
- **End** - Zum Ende der Datei springen (lädt Tail bei Bedarf)
- **Esc** - Vorschau schließen
- Info-Leiste zeigt: Line Position | Mode (FWD/BWD) | Status (FULL/CHUNK) | Chunks geladen | Dateigröße

## Keyboard Shortcuts

### MC-Style Function Keys (Kontextabhängig)

| Taste | Funktion | Kontext | Beschreibung |
|-------|----------|---------|--------------|
| **F1 / ?** | Help | Alle | Zeigt Hilfe an |
| **F2** | Sort | Alle | Sortierung (Name, Size, Date) |
| **F3** | View/Edit | ProfileList: Edit Profile<br>BucketList: Edit Bucket<br>S3/Filesystem: View File | Kontextabhängig: Edit Config oder View File |
| **F4** | Filter | Alle | Filtert Items nach Namen |
| **F5** | Copy | S3/Filesystem | Kopiert zwischen Panels |
| **F6** | Rename | S3/Filesystem | Benennt Datei/Ordner um |
| **F7** | Create | BucketList: Bucket Config<br>S3/Filesystem: Mkdir | Kontextabhängig: Config oder Ordner erstellen |
| **F8 / Del** | Delete | Alle | Löscht ausgewähltes Item |
| **F9** | Advanced | Alle | Schaltet Advanced Mode um (erweiterte Infos) |
| **F10 / q** | Quit | Alle | Beendet Anwendung |
| **F12** | Toggle FS | Alle | Wechselt zu lokalem Filesystem |

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
          "base_prefix": "subfolder/",
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
- `base_prefix` - Optionaler Start-Prefix beim Öffnen des Buckets (z.B. "subfolder/" oder "logs/2024/")
  - Ermöglicht direktes Navigieren zu einem Unterordner
  - Bucket öffnet automatisch im angegebenen Prefix
  - Nützlich für organisierte Buckets mit vielen Unterordnern
  - Notwendig bei Berechtigungen auf bestimmte Prefixe
- `description` - Optionale Beschreibung
- `role_chain` - Optionale Liste von Role ARNs für Role Chaining
- `endpoint_url` - Custom S3 Endpoint für S3-kompatible Services (optional)
- `path_style` - Force Path-Style URLs für Minio, Ceph, etc. (optional, default: false)

## S3-kompatible Services

s3c funktioniert mit allen S3-kompatiblen Object Storage Services:
- 🇩🇪 **Hetzner Object Storage**
- 🏠 **Minio** (self-hosted)
- 🌊 **DigitalOcean Spaces**
- ☁️ **Azure Blob Storage** (mit S3-Kompatibilität)
- 🗄️ **Ceph S3 Gateway**
- 📦 **Wasabi**
- 🔧 **Andere S3-kompatible Services**

### Credentials für S3-kompatible Services

Die Access Keys dieser Services werden im **gleichen Format** wie AWS Credentials gespeichert:

```bash
# ~/.aws/credentials

# AWS Profil
[aws-production]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

# Hetzner Object Storage
[hetzner]
aws_access_key_id = HETZNER_ACCESS_KEY_123
aws_secret_access_key = hetzner_secret_key_456

# Minio (self-hosted)
[minio-local]
aws_access_key_id = minioadmin
aws_secret_access_key = minioadmin

# DigitalOcean Spaces
[do-spaces]
aws_access_key_id = DO_SPACES_KEY_ABC
aws_secret_access_key = do_spaces_secret_xyz
```

**Wichtig:** Die Feldnamen bleiben `aws_access_key_id` und `aws_secret_access_key`, aber die **Werte** sind die Access Keys des jeweiligen Services.

### Beispiel-Konfigurationen

**Hetzner Object Storage:**
```json
{
  "profiles": [
    {
      "name": "hetzner",
      "description": "Hetzner Storage",
      "buckets": [
        {
          "name": "my-backup-bucket",
          "region": "fsn1",
          "endpoint_url": "https://fsn1.your-objectstorage.com",
          "description": "Backup Storage"
        }
      ]
    }
  ]
}
```

**Minio (self-hosted):**
```json
{
  "profiles": [
    {
      "name": "minio-local",
      "description": "Internal Minio",
      "buckets": [
        {
          "name": "company-data",
          "region": "us-east-1",
          "endpoint_url": "https://minio.company.com:9000",
          "path_style": true,
          "description": "Company Internal Storage"
        }
      ]
    }
  ]
}
```

**DigitalOcean Spaces:**
```json
{
  "profiles": [
    {
      "name": "do-spaces",
      "description": "DigitalOcean Spaces",
      "buckets": [
        {
          "name": "my-space",
          "region": "fra1",
          "endpoint_url": "https://fra1.digitaloceanspaces.com",
          "description": "Frankfurt Space"
        }
      ]
    }
  ]
}
```

**Azure Blob Storage (mit S3-Kompatibilität):**
```json
{
  "profiles": [
    {
      "name": "azure",
      "description": "Azure Storage",
      "buckets": [
        {
          "name": "my-container",
          "region": "westeurope",
          "endpoint_url": "https://myaccount.blob.core.windows.net",
          "description": "Azure West Europe"
        }
      ]
    }
  ]
}
```

### Region-Namen der verschiedenen Services

**Wichtig:** Die Region ist **nur bei AWS S3 wirklich relevant**. Bei alternativen Anbietern mit Custom Endpoint bestimmt der Endpoint die tatsächliche Verbindung - die Region ist nur ein Pflichtfeld für das AWS SDK.

| Service | Region-Format | Bedeutung | Beispiele |
|---------|---------------|-----------|-----------|
| **AWS S3** | AWS Regions | ✅ **Wichtig** - bestimmt Routing | `eu-west-1`, `us-east-1`, `ap-southeast-1` |
| **Hetzner** | Beliebig | ⚠️ Dummy - Endpoint zählt | `fsn1`, `nbg1` oder einfach `us-east-1` |
| **DigitalOcean Spaces** | Beliebig | ⚠️ Dummy - Endpoint zählt | `fra1`, `nyc3` oder einfach `us-east-1` |
| **Minio/Ceph** | Beliebig | ⚠️ Dummy - Endpoint zählt | `us-east-1` (Standard) |
| **Wasabi** | Wasabi Regions | ⚠️ Dummy - Endpoint zählt | `eu-central-1`, `us-east-1` |

**Technischer Hintergrund:**
- Bei **AWS S3** ohne Custom Endpoint: Region bestimmt, zu welchem AWS-Rechenzentrum verbunden wird
- Bei **Custom Endpoint** (Hetzner, Minio, etc.): Der Endpoint-URL bestimmt die Verbindung, die Region ist nur ein Pflichtfeld für das AWS SDK und hat keine Routing-Funktion

### Path-Style URLs: Wann ist es nötig?

**Path-Style** ändert das URL-Format für S3-Requests:

| URL-Style | Format | Beispiel |
|-----------|--------|----------|
| **Virtual-hosted-style** (Standard) | `https://bucket-name.endpoint.com/key` | `https://my-bucket.s3.amazonaws.com/file.txt` |
| **Path-style** | `https://endpoint.com/bucket-name/key` | `https://s3.amazonaws.com/my-bucket/file.txt` |

**Wann muss Path-Style aktiviert werden?**

| Service | Path-Style nötig? | Grund |
|---------|------------------|-------|
| **AWS S3** | ❌ Nein | Unterstützt beides, Virtual-hosted ist Standard |
| **Hetzner** | ❌ Nein | Unterstützt Virtual-hosted-style |
| **DigitalOcean Spaces** | ❌ Nein | Unterstützt Virtual-hosted-style |
| **Wasabi** | ❌ Nein | Unterstützt Virtual-hosted-style |
| **Minio** | ✅ **Ja** | Benötigt oft Path-style URLs |
| **Ceph S3 Gateway** | ✅ **Ja** | Benötigt oft Path-style URLs |

**Empfehlung:** Nur aktivieren wenn der Service-Provider es explizit verlangt oder wenn Verbindungsfehler auftreten.

### Wie es funktioniert

1. **Credentials:** Service-spezifische Access Keys in `~/.aws/credentials` speichern
2. **Profil:** In s3c wie ein normales AWS-Profil verwenden
3. **Endpoint:** Custom `endpoint_url` in Bucket-Konfiguration setzen
4. **Region:** Korrekte Region des Services angeben (siehe Tabelle oben)
5. **API:** AWS SDK sendet Requests an Custom Endpoint (gleiche S3 API)
6. **Nutzung:** Identische Bedienung wie mit AWS S3

**Vorteil:** Ein Tool für alle S3-kompatiblen Services! 🎯

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

### Design Pattern: The Elm Architecture (TEA)

s3c folgt **The Elm Architecture (TEA)** für eine saubere, wartbare Codebase:

- **Model** (`app/state.rs`) - Anwendungszustand und Datenmodelle
- **Message** (`message.rs`) - Alle möglichen Aktionen als Enum
- **Update** (`app/update.rs`) - Zentrale State-Update-Logik
- **View** (`ui/`) - Reine Rendering-Funktionen

**Vorteile:**
- ✅ Vorhersagbarer State-Flow
- ✅ Einfaches Testing
- ✅ Klare Trennung von Logik und UI
- ✅ Erweiterbar für zukünftige Features

### Module

- **`src/main.rs`** - Application Setup und Teardown
- **`src/models/`** - Datenmodelle (Config, List)
- **`src/app/`** - TEA Core (State, Update, Navigation)
- **`src/operations/`** - Business Operations (Run-Loop, S3, File, etc.)
- **`src/handlers/`** - Input-zu-Message Konvertierung
- **`src/ui/`** - TUI-Komponenten und Rendering

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
01Help  02Sort  03View/Edit  04Filter  05Copy  06Rename  07Mkdir/Config  08Delete  09Advanced  10Exit
```
- Kontextabhängige Funktionen (ändern sich je nach Panel-Typ)
- F3: Edit (Profile/Bucket) oder View (S3/Filesystem)
- F7: Mkdir (S3/Filesystem) oder Config (BucketList)
- F9: Toggle Advanced Mode (zeigt erweiterte Informationen)
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
