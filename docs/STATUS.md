**Objetivo inicial**

- Comunicador desktop leve, com comunidades, amigos, mensagens, voz e mídia, sem depender de um servidor de aplicação para todas as conversas.

**Objetivo já atingido**

- Interface Dioxus Desktop, perfis, amizades, servidores, canais e conversas diretas estruturados.
- Histórico SQLite, outbox, recibos e mecanismos de comunicação P2P implementados.
- Transporte QUIC/Opus duplex validado com áudio sintético em dois processos locais.

**Entregas realizadas**

- Client desktop em Rust + Dioxus.
- Navegação de Início, Amigos, DMs e servidores.
- Mensagens, busca no histórico e recibos integrados ao aplicativo.
- Workers de voz privada e coletiva integrados ao código, ainda sem homologação audível entre computadores.
- Build portátil de prévia para Windows; não considerada estável.

**Entregas propostas**

- Homologar mensagens, recibos, DMs e chamadas entre computadores reais.
- Homologar voz coletiva em sala com três Clients.
- Vídeo, compartilhamento de tela e áudio do sistema.
- Melhorar eventos, anexos, notificações e distribuição do Client.

**Etapas realizadas**

- Fundação do aplicativo, armazenamento e identidade local.
- Servidores, amizades, canais e apresentação Dioxus.
- Laboratório duplex QUIC/Opus; integração inicial da mídia ao Client.
- DMs, chamadas privadas e ajustes da lateral de amigos.

**Etapas propostas**

- Validar o fluxo completo Client A → Client B, incluindo recibos e chamada privada.
- Validar canal coletivo com múltiplos participantes.
- Refinar UX real, mídia e distribuição Windows.

**Progresso desta etapa**

- Documentação pública reduzida e prototipagem baseada no Dioxus.
- Pipeline de empacotamento restrito ao Client para exportação pública.

**Progresso atual**

- Base de desenvolvimento: H5.0-B5R.3.
- Compilação local do Client reportada; homologação entre dispositivos pendente.
- Sem lançamento público estável ou assinatura digital confirmados.
