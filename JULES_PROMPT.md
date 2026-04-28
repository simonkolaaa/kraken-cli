<instruction>You are an expert software engineer. You are working on a WIP branch. Please run `git status` and `git diff` to understand the changes and the current state of the code. Analyze the workspace context and complete the mission brief.</instruction>
<workspace_context>
</workspace_context>
<mission_brief>["Il bot non sta ancora eseguendo trade autonomi, risulta troppo conservativo. Dobbiamo allentare drasticamente i filtri per vedere l'operatività reale:

Allarga il Filtro Tecnico: Modifica i limiti dell'RSI. Ora scarta gli asset <30 o >70. Cambialo in modo che accetti asset con RSI tra 50 e 80.

Soglia LLM Abbassata: Riduci la soglia minima di confidenza (confidence) per un segnale BUY/SELL al 55%

Gestione News Vuote (Fondamentale): Attualmente, se un'altcoin non ha news recenti, l'LLM probabilmente restituisce HOLD. Modifica la logica: se l'API delle news restituisce 0 articoli per un asset, fai in modo che l'LLM valuti comunque l'acquisto basandosi anche sull'analisi tecnica (es. passando all'LLM l'RSI e la SMA), oppure abbassa ulteriormente le difese in assenza di notizie negative.

Aggiornamento Prompt di Sistema: Nel prompt inviato a OpenRouter, aggiungi l'istruzione: 'Sei un trader aggressivo ma non esagerare. Sii propenso al rischio senza esagerare. Se c'è una positività tecnica o di news, favorisci un segnale di BUY'.

Implementa queste modifiche per massimizzare le probabilità di trade e fai il push su GitHub."]</mission_brief>