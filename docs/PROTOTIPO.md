**Interface atual — Rust + Dioxus Desktop**

```text
┌─────────────────────────────────────────────────────────────────────┐
│ BoboCord                         Pesquisar                      ─ □ × │
├───────┬──────────────────────┬──────────────────────────────────────┤
│Início │ Amigos / Canais      │ Conversa ou canal                   │
│       │                      │                                      │
│Servidor│ Amigos              │ Histórico, anexos e mensagens       │
│       │ DMs                 │                                      │
│  +    │ Voz / Texto          │                                      │
│       │                      ├──────────────────────────────────────┤
│       │ Perfil / voz         │ Mensagem...                          │
└───────┴──────────────────────┴──────────────────────────────────────┘
```

- **Início:** panorama de amigos, conversas e servidores.
- **Amigos:** abas Todos, Online, Pendentes e Bloqueados; adicionar amigo.
- **DMs:** lateral persistente, conversa no painel principal e ligação privada com aceitar/recusar.
- **Servidores:** lista de setores e canais de texto/voz; membros quando necessário.
- **Voz de servidor:** entrar e sair da sala, sem solicitação de aceite a cada participante.
- **Creator:** diagnósticos e ferramentas internas fora do Client público.
- **Referências:** Discord e [griasdi](https://griasdi.github.io/)
