# Kraken Algorithmic Trading Bot

![version](https://img.shields.io/badge/version-1.0.0-blue)
![license](https://img.shields.io/badge/license-MIT-green)
![platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-lightgrey)

## 1. Descrizione Generale
Il progetto è un Bot di Trading Algoritmico scritto in Rust, progettato per operare autonomamente sull'exchange Kraken. Utilizza l'analisi del sentiment in tempo reale, generata tramite AI partendo dalle notizie di mercato, per prendere decisioni di trading ottimali.

## 2. Stack Tecnologica
- **Linguaggio**: Rust (con `tokio` per l'asincronia).
- **Dashboard**: Web server leggero basato su `axum` (in esecuzione sulla porta `3000`), con interfaccia grafica "Hacker Terminal" (testo verde neon su sfondo nero) per il monitoraggio in tempo reale.
- **News Engine**: Integrazione nativa con le API di **CryptoCompare** per il recupero puntuale delle ultime notizie crypto.
- **LLM Engine**: Integrazione con **OpenRouter** (modello `meta-llama/llama-3-8b-instruct:free`) per eseguire la *Sentiment Analysis* contestuale sui titoli delle news recuperate.
- **Database/State**: La gestione dello stato è asincrona e thread-safe. Utilizza `Arc<RwLock<PaperState>>` per prevenire deadlock tra il loop principale di trading e il web server della dashboard.

## 3. Funzionamento
Il bot esegue un ciclo continuo 24/7 operando in modo del tutto autonomo:
- **Recupero News**: Ad ogni ciclo di clock il bot legge le ultime notizie di mercato.
- **Valutazione Sentiment**: L'AI legge le news, considera i saldi attuali e determina una decisione logica (Decisioni possibili: `BUY`, `SELL`, `HOLD`).
- **Esecuzione Ordini**: Attualmente configurato in modalità **Paper Trading** con un saldo virtuale iniziale di `10.000 USD`. Le esecuzioni sono calcolate sui prezzi di mercato live senza rischio finanziario.
- **Notifiche**: Aggiornamenti asincroni via Telegram Bot API per avvii, esecuzione dei trade ed eventuali attivazioni del *Kill Switch* di emergenza.

## 4. Configurazione
Per avviare il bot, è necessario creare un file `.env` nella root del progetto contenente le seguenti variabili d'ambiente:

```env
# Chiavi API Kraken (per uso in live)
KRAKEN_API_KEY=la_tua_chiave_kraken
KRAKEN_API_SECRET=il_tuo_secret_kraken

# Telegram Bot
TELEGRAM_BOT_TOKEN=il_tuo_token_botfather
TELEGRAM_CHAT_ID=il_tuo_chat_id

# News e AI
CRYPTOCOMPARE_API_KEY=la_tua_chiave_cryptocompare
OPENROUTER_API_KEY=la_tua_chiave_openrouter
```

## 5. Deployment
Il bot è strutturato per essere eseguito come demone su server Linux (es. AWS EC2).

1. **Accesso al server e clona la repo**:
   ```bash
   git clone <il_tuo_repo>
   cd kraken-cli
   ```
2. **Setup dell'ambiente**:
   Assicurati di aver configurato Rust e di aver creato il file `.env`.
3. **Avvio in persistenza con `tmux`**:
   Per evitare che il bot si spenga chiudendo la sessione SSH:
   ```bash
   tmux new -s krakenbot
   cargo run --release -- bot start
   ```
   Puoi disconnetterti da tmux premendo `Ctrl+B` seguito da `D`. Per ricollegarti: `tmux attach -t krakenbot`.

---
*Disclaimer: Il software è fornito a scopo didattico e sperimentale.*
