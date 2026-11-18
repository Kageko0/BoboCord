<p align="center">
  <strong>BoboCord</strong><br>
  Converse com amigos, participe de comunidades e crie seus espaços.
</p>

<p align="center">
  <a href="docs/STATUS.md">Progresso</a> ·
  <a href="docs/PROTOTIPO.md">Interface</a> ·
  <a href="https://github.com/Kageko0/BoboCord/releases">Downloads</a>
</p>

BoboCord é um comunicador desktop em desenvolvimento, inspirado na organização de comunidades e conversas do Discord. O Client usa Rust e Dioxus Desktop, com histórico local e comunicação P2P quando possível.

- **Amigos e mensagens diretas:** interface e integração implementadas; entrega entre computadores ainda pendente de homologação.
- **Servidores e canais:** organização e navegação presentes no Client.
- **Voz privada e coletiva:** transporte QUIC/Opus implementado; conversas audíveis entre usuários ainda pendentes de validação real.
- **Vídeo e tela compartilhada:** propostas, não entregues.

**Baixar:** consulte [Releases](https://github.com/Kageko0/BoboCord/releases). Ainda não há uma versão B5R.3 homologada para distribuição pública. Quando existir, baixe somente o pacote **Client** da versão mais recente.

<details>
<summary>Como usar a versão portátil (Windows)</summary>

1. Extraia todo o ZIP do Client.
2. Abra `BoboCord.exe`.
3. Se necessário, instale o Microsoft Edge WebView2 Runtime e os componentes Visual C++ exigidos pelo pacote.

O aplicativo mantém os dados no perfil local do usuário. Esta fase não dispõe de atualização automática nem de instalador oficial.
</details>

O andamento, as entregas e as próximas etapas estão em [Status](docs/STATUS.md). A referência visual do Client está em [Protótipo](docs/PROTOTIPO.md).

**Licença:** ainda não definida. Não confundir a disponibilidade do executável com autorização para redistribuição ou modificação do código-fonte.
