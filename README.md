# BoboCord

> Comunicação entre amigos, equipes e comunidades pequenas, com foco em leveza, privacidade e conexões peer-to-peer.

**Estado:** v0.1 Foundation · Dioxus Desktop + Fase F1 local (pré-alfa) · **Plataformas iniciais:** Windows e Linux · **Próxima plataforma:** Android · **Linguagem principal:** Rust

BoboCord é um aplicativo open source de comunicação inspirado na organização de canais do Discord e na colaboração do Teams, mas projetado para grupos pequenos e previsíveis. O objetivo é combinar texto, voz, câmera, compartilhamento de tela e ferramentas úteis numa aplicação desktop leve, com serviços independentes e controle local dos dados.

**Princípio central:** P2P sempre que viável; infraestrutura mínima quando necessária. BoboCord **não promete operação 100% sem servidores**: descoberta de peers, NAT restritivo, entrega offline e retransmissão ocasional podem exigir serviços auxiliares.

## Objetivos

- Manter baixo consumo de memória, CPU, GPU e banda quando o aplicativo estiver em repouso.
- Oferecer **espaços lógicos** (chamados de *servidores* na interface), canais de texto e voz, membros e permissões — sem exigir uma VM para cada espaço.
- Sincronizar mensagens entre participantes com histórico local e entrega posterior quando possível.
- Dar prioridade a conexões diretas e usar relay apenas quando necessário.
- Isolar captura, mídia, integrações e extensões para que possam ser ativadas sob demanda.
- Compartilhar o núcleo Rust entre Windows, Linux e, posteriormente, Android.
- Ter uma base limpa, testável e documentada para contribuição via GitHub.

## Funcionalidades planejadas

| Área | Escopo | Momento |
| --- | --- | --- |
| Identidade | Perfil, avatar, status, dispositivos e convites | MVP |
| Espaços e canais | Servidores lógicos, canais, membros e permissões | MVP |
| Chat | Texto, menções, reações, histórico e sincronização | MVP |
| Voz | Salas de voz, push-to-talk, mute, volume individual, Opus | Após MVP |
| Tratamento de áudio | Supressão de ruído, ganho e cancelamento de eco | Após voz |
| Vídeo | Webcam e compartilhamento de monitor/janela | Etapa posterior |
| BoboReplay | Buffer circular dos últimos 50 s, gravação e atalhos | Etapa posterior |
| Música | Fila compartilhada de links/playlists e integrações permitidas | Etapa posterior |
| Organização | Eventos, lembretes e notificações | Etapa posterior |
| Visual | Temas, ícones, avatares, WebP e stickers | Evolutivo |
| Extensões | API versionada, bots e plugins isolados | Futuro |
| Android | Cliente adaptado, reutilizando núcleo/protocolo | Futuro |

> Reprodução de YouTube e Spotify deverá respeitar APIs, players oficiais e políticas de licenciamento. Não estão previstos extração não autorizada de áudio, retransmissão de faixas protegidas ou reprodução oculta que viole seus termos.

## Diretrizes de arquitetura

```text
+------------------------------------------------+
| BoboCord Desktop                               |
| Dioxus Desktop (RSX + CSS / WebView do sistema) |
+------------------------+-----------------------+
                         |
+------------------------v-----------------------+
| BoboCore (Rust)                                |
| identidade, espaços, canais, estado e eventos   |
+-------+----------------+-----------------------+
        |                |                       |
+-------v------+ +-------v---------+ +-----------v----+
| bobo-network | | bobo-storage    | | Módulos opcionais|
| P2P / QUIC   | | SQLite / cache  | | voz / vídeo /    |
|             | |                 | | captura / bots   |
+-------+------+ +-----------------+ +----------------+
        |
+-------v----------------------------------------+
| Serviços auxiliares opcionais                  |
| descoberta / relay / fila offline / arquivos   |
+------------------------------------------------+
```

### Tecnologias candidatas

- **Rust:** núcleo, serviços e contratos.
- **Dioxus Desktop 0.7.10:** UI RSX com CSS tokenizado, WebView2 no Windows e WebKitGTK no Linux.
- **Tokio:** tarefas assíncronas e cancelamento.
- **Iroh/QUIC:** conexões diretas, descoberta/relay conforme capacidades da versão selecionada.
- **SQLite:** armazenamento local; migrações versionadas.
- **WebRTC + Opus:** voz, vídeo e mídia em tempo real, após prova de conceito.
- **RNNoise ou WebRTC APM:** processamento de microfone a avaliar por desempenho e integração.
- **WebP:** imagens e stickers; cache de texturas e animação sob demanda.
- **WGSL:** shaders opcionais, não dependência para componentes básicos da interface.

A stack será consolidada após testes mínimos de compilação em Windows e Linux. **Não acoplar o domínio a uma biblioteca de rede, UI ou mídia específica.**

## P2P e persistência

Cada pessoa possui uma identidade criptográfica por dispositivo. Espaços e canais são objetos lógicos, não processos permanentemente hospedados. Os peers tentam conexão direta; em redes que impedem isso, um relay pode ser necessário.

A base local guarda mensagens, perfis, preferências e eventos. A sincronização deve lidar com mensagens repetidas, reconexão, ordem parcial de eventos e conflitos. A entrega de conteúdo para pessoas completamente offline **não é garantida** sem algum peer disponível ou serviço de armazenamento/entrega.

A segurança deve considerar autenticação entre peers, criptografia de mensagens, rotação/revogação de chaves, convites com expiração e autorização efetiva para operações de canais e membros. Não confundir criptografia de transporte com criptografia ponta a ponta de dados persistidos.

## Oracle Cloud (opcional)

Usar serviços gratuitos **somente dentro das cotas e condições vigentes** como apoio a descoberta, arquivos temporários, mensagens pendentes e recuperação. O aplicativo não deve depender da Oracle para abrir, mostrar histórico local ou funcionar em LAN.

Políticas iniciais **configuráveis, não garantias de capacidade**:

- Clips: armazenamento local por padrão; upload somente por ação explícita.
- Vídeos/arquivos temporários: expiração sugerida de 7 dias.
- Limite sugerido por envio: 100 MB, sujeito a cota e banda.
- Metadados e arquivos enviados à nuvem: criptografia e validação de acesso.
- Limpeza automatizada por política de retenção, com tratamento de falhas.

Nenhuma credencial da Oracle deve ficar no cliente distribuído ou no repositório.

## BoboReplay (planejado)

Capturar uma janela ou monitor escolhido, codificar preferencialmente via hardware e manter um **buffer circular de pacotes comprimidos**, preservando referências necessárias de áudio/vídeo. Ao acionar um atalho, salvar até 50 segundos num contêiner adequado. Captura de áudio do microfone, do sistema e da chamada deve ter controles e faixas separados quando a plataforma permitir. Respeitar permissões e limitações de cada sistema operacional. Não implementar na primeira entrega.

## Metas de desempenho (não comprovadas)

| Indicador | Meta de referência |
| --- | --- |
| RAM em repouso | 80–150 MB |
| CPU em repouso | abaixo de 1% médio, em máquina de referência |
| Inicialização quente | abaixo de 2 s |
| Interface | responsiva, até 60 FPS quando necessário |
| Replay | janela configurável, 50 s como padrão |
| Voz | medir latência ponta a ponta e perdas em redes reais |

Estas são hipóteses a validar com medições, não promessas do produto. Nenhum módulo inativo deve executar loops de polling excessivos.

## Plano de entregas

1. **v0.1 — Foundation:** workspace Rust, UI Dioxus Desktop (migração Fase E), temas, navegação, espaços/canais locais, SQLite, testes e CI.
2. **v0.2 — P2P Text:** identidade por dispositivo, convites, conexão de dois computadores, mensagens persistentes, reconexão e deduplicação.
3. **v0.3 — Voice:** salas de voz pequenas, Opus, controles de áudio, cancelamento de tarefas e medições.
4. **v0.4 — Video:** câmera, compartilhamento de tela e qualidade adaptativa.
5. **v0.5 — BoboReplay:** captura, buffer e exportação local.
6. **v0.6 — Cloud & Features:** armazenamento auxiliar, mídia temporária, eventos, personalização e fila de links.
7. **v0.7+ — Extensions & Android:** API de plugins, bots e cliente Android.

### Critérios de aceitação da v0.1

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` e `cargo test --workspace` executam com sucesso.
- Aplicativo abre em Windows e Linux (validado por build/teste, ou limitações documentadas se o runner estiver indisponível).
- Há layout com lista de espaços, canais, painel de mensagens e configurações.
- Criar/renomear/excluir espaço e canal persiste após reinício.
- Histórico de mensagens **locais** funciona, com IDs estáveis e timestamps.
- Tema padrão e alternativo são aplicáveis e persistidos.
- Ausência de servidor, Internet ou credenciais não impede o uso da v0.1.
- Módulos de voz, vídeo, replay e Oracle **não são simulados como funcionais**.

## Organização atual da Foundation

```text
bobocord/
├── Cargo.toml
├── README.md
├── AGENTS.md
├── apps/
│   └── desktop/           # componentes RSX, estado e assets CSS embutidos
├── crates/
│   ├── bobo-app/          # ações tipadas, seleção, rascunhos e worker
│   ├── bobo-core/         # domínio preservado
│   └── bobo-storage/      # SQLite/001 preservada + 002 de estado local
├── scripts/              # testes nativos opcionais e medição de processos
├── docs/                 # validação e histórico da migração
└── .github/workflows/ci.yml
```

Os demais crates serão criados **apenas quando usados**. Evitar diretórios vazios e abstrações prematuras.

## Privacidade e segurança

- Informar quando microfone, câmera e captura estiverem ativos.
- Não enviar telemetria sem consentimento.
- Conservar dados localmente por padrão; uploads explícitos.
- Evitar logging de conteúdo de mensagens, tokens e dados pessoais.
- Documentar claramente limites de segurança das primeiras versões.
- Usar bibliotecas consolidadas para criptografia; não criar criptografia própria.

## Desenvolvimento

A **v0.1 Foundation** implementa somente texto local. Os recursos descritos como
planejados nas seções anteriores continuam fora desta entrega. As instruções para
agentes estão em [AGENTS.md](AGENTS.md); o registro dos checks e limitações está em
[docs/VALIDATION.md](docs/VALIDATION.md). O escopo da migração está em
[CODEX_NEXT_STEPS.md](CODEX_NEXT_STEPS.md). O escopo de qualidade local executado
é somente a **Fase F1** de [BOBOCORD_CODEX_FGH.md](BOBOCORD_CODEX_FGH.md);
as fases G/H permanecem futuras.

### O que funciona

- Criar, renomear e excluir espaços e canais de texto; exclusões exigem confirmação.
- Salvar mensagens locais e navegar pelo histórico de cada canal, com timestamps UTC.
- Editar o nome do perfil local e alternar os temas claro/escuro nas configurações.
- Persistir os dados no SQLite e recarregá-los ao abrir o aplicativo.
- Mostrar erros de validação/armazenamento e tentar novamente por **Recarregar**.
- Restaurar espaço/canal e rascunhos por canal ao reiniciar, com autosave e flush
  no encerramento normal.
- Paginar o histórico no SQLite por cursor estável, sem consulta integral ou OFFSET.
- Carregar anteriores mantendo a posição de leitura e acompanhar novas mensagens
  somente quando a pessoa já está perto do fim.
- Menu contextual para espaços/canais por botão direito ou Shift+F10, com setas
  e Escape; formulários com descrições e erros acessíveis.

O banco começa sem espaços, canais ou mensagens. Apenas o perfil local `Você` e
o tema escuro são inicializados pela migração. A interface não contém dados de
exemplo permanentes. Avatar usa placeholder textual; não há editor de avatar.

### Toolchain e dependências

Use Rust estável **1.95 ou superior**, edição 2024. A versão foi escolhida por ser
o toolchain disponível e testado, com o conjunto de dependências do `Cargo.lock`.
`rust-toolchain.toml` usa `stable`; a CI usa 1.95.0 para manter a referência testada.
O lockfile deve ser versionado e os builds reproduzidos com `--locked`.

Dependências diretas: Dioxus/dioxus-desktop **0.7.10** (fixadas), async-channel 2.5,
rusqlite 0.37 com SQLite embutido, UUID v4, chrono e thiserror. tempfile, pollster e
dioxus-ssr são usados nos testes. O frontend usa o WebView mantido do sistema,
sem Blitz/WGPU ou servidor de aplicação. RSX e CSS são embutidos no executável;
`cargo run` funciona sem CLI `dx`, Node, npm, CDN ou fontes remotas.
Veja a [documentação Desktop do Dioxus](https://dioxuslabs.com/learn/0.7/guides/platforms/desktop/).

As features padrão do renderer foram desabilitadas (incluindo o devserver Dioxus), mas
`tokio_runtime` é necessário no Dioxus Desktop 0.7.10: o renderer usa primitivas
Tokio e obtém o runtime ao abrir o WebView. A entrada oficial cria esse runtime;
a aplicação não cria outro, e domínio/SQLite continuam sem dependência de Tokio.
Isso foi confirmado por compilação e abertura real, não por conveniência.
Outra incompatibilidade da 0.7.10 aparece em release: referências não protegidas
a métodos do inspetor. A feature `wry/devtools` foi habilitada apenas para esses
métodos compilarem, sem habilitar o devserver/hot-reload Dioxus. Nenhuma porta de
depuração é aberta pelo aplicativo.

### Windows (PowerShell)

Pré-requisitos: Rust via rustup, **Visual Studio Build Tools** com a carga
**Desenvolvimento para desktop com C++** (MSVC e Windows SDK). O build MSVC foi
confirmado neste ambiente; `cl.exe` não precisa estar no PATH do PowerShell
quando o toolchain consegue localizar o Build Tools. Não é necessário instalar
SQLite separadamente. Também é necessário o **Microsoft Edge WebView2 Runtime**
Evergreen (157.0.4341.42 no ambiente validado). Se ausente, instale-o pelos
[distribuidores oficiais da Microsoft](https://developer.microsoft.com/microsoft-edge/webview2/).

```powershell
rustc --version
cargo --version
cargo build --workspace --locked
cargo run -p bobocord-desktop --locked
cargo build --workspace --release --locked
```

O executável de desenvolvimento também pode ser iniciado diretamente:

```powershell
.\target\debug\bobocord-desktop.exe
# ou .\target\release\bobocord-desktop.exe
```

Por padrão o banco fica em `%LOCALAPPDATA%\BoboCord\bobocord.sqlite3`.
Para usar um diretório de teste ou portátil:

```powershell
$env:BOBOCORD_DATA_DIR = Join-Path $PWD 'target\meus-dados'
cargo run -p bobocord-desktop --locked
```

### Linux (Ubuntu/Debian; não verificado localmente)

Comandos propostos; a CI contém um job Ubuntu, mas ainda não foi executada nesta
entrega. É necessária uma sessão gráfica X11 ou Wayland para abrir a janela.
Os testes de Rust usam VirtualDom/headless e não precisam de display; compilar o
pacote desktop ainda requer os headers e bibliotecas GTK/WebKit.

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libgtk-3-dev \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev libxdo-dev libssl-dev
cargo build --workspace --locked
cargo run -p bobocord-desktop --locked
```

As bibliotecas propostas cobrem GTK3/WebKitGTK 4.1 e integração desktop;
confira os [requisitos oficiais do Dioxus](https://dioxuslabs.com/learn/0.7/getting_started/#platform-specific-dependencies).
O banco fica em `$XDG_DATA_HOME/BoboCord/bobocord.sqlite3` quando a variável é um
caminho absoluto, ou em `$HOME/.local/share/BoboCord/bobocord.sqlite3`.

```bash
BOBOCORD_DATA_DIR=./target/meus-dados cargo run -p bobocord-desktop --locked
```

`BOBOCORD_DATA_DIR` aceita caminho relativo ao diretório de execução. As pastas
são criadas automaticamente pela thread de armazenamento. Não há caminhos de
Windows codificados na camada compartilhada.

### Verificação

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --locked
```

O workflow está configurado para executar esses checks em Windows e Ubuntu,
incluindo build release; nenhum run remoto foi executado nesta sessão. No ambiente desta
entrega, os comandos também foram executados com `--offline`: o cache de crates
já estava disponível. Uma primeira compilação em outra máquina normalmente
precisa de Internet para baixar crates; **a execução do aplicativo não precisa**.
Não há telemetria nem comunicação externa no código da aplicação. O WebView usa
protocolos locais do Dioxus; WebView2 pode manter processos e cache próprios do
runtime do sistema. O perfil WebView fica em `webview/` ao lado do banco, sem
substituir ou mover o arquivo SQLite existente.

### Teste nativo opcional (Windows)

Além dos testes Rust, `scripts/desktop_smoke.py` exercita o DOM real via protocolo
de depuração do WebView2: CRUDs, erros, mensagens, persistência após reinício,
temas, perfil, redimensionamento e escalas simuladas. Requer Python 3 e usa apenas
a biblioteca padrão. **Use um diretório novo exclusivo para testes**: o script
cria e exclui dados de fixture. Não execute contra seu banco pessoal.

```powershell
$env:BOBOCORD_DATA_DIR = Join-Path $PWD 'target\ui-smoke-novo'
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9223'
cargo run -p bobocord-desktop --locked
# Em outro terminal na raiz:
python scripts/desktop_smoke.py flow
# Feche normalmente a janela e reabra com o mesmo diretório/ambiente; depois:
python scripts/desktop_smoke.py reopen
# Com a janela ainda aberta, teste teclado, foco e reduced motion:
python scripts/desktop_smoke.py keyboard
```

A porta de depuração é ativada somente por essa variável no teste. Remova-a do
terminal (`Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`) ao terminar.
O app não habilita depuração remota em uma execução normal. `dx` não estava
instalado no ambiente; foi usada a execução equivalente por Cargo/executável.

`scripts/measure-desktop.ps1 -BoboProcessId <PID> -Label debug` mede por 10 segundos
o processo escolhido e sua árvore WebView2 (working set, bytes privados e CPU).
Execute separadamente com `-Label release`. A soma de working sets pode contar
páginas compartilhadas mais de uma vez; as amostras não são benchmarks.

Testes F1 adicionais (Windows, sempre em diretório novo de fixture):

```powershell
python scripts/seed_load.py target\f1-carga-nova
$env:BOBOCORD_DATA_DIR = Join-Path $PWD 'target\f1-carga-nova'
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9223'
cargo run -p bobocord-desktop --locked
# Em outro terminal na raiz, com a janela aberta:
python scripts/f1_smoke.py scroll
python scripts/f1_smoke.py context
python scripts/f1_smoke.py write-close <PID-do-app>
# Reabra o app com o mesmo diretório/ambiente; depois:
python scripts/f1_smoke.py read
```

`seed_load.py` recusa sobrescrever banco existente; gera 10 mil mensagens em
quatro canais (9 mil no canal ativo). `startup_f1.py <exe> <diretório-em-target>`
mede abertura até o renderer pronto, incluindo o overhead do probe CDP local,
e fecha o app normalmente. Ele aceita somente diretórios em `target/`.

### Fronteiras e dados

```text
apps/desktop (componentes RSX, sinais independentes e CSS tokenizado)
  -> bobo-app (Action/Request/Reply, seleção, rascunhos e thread SQLite)
    -> bobo-core (Service, entidades, validação, contrato Repository)
      -> bobo-storage (implementação Repository, SQLite e migrações)
```

`bobo-core` não importa Dioxus nem rusqlite. A UI não executa SQL. O acesso ao
banco, inclusive sua abertura/migração, acontece em uma thread dedicada que
aguarda comandos em canal bloqueante; não há polling ou timer de renderização.
O retorno é aguardado por um future Dioxus em `async-channel`, sem polling.
Navegação, histórico, perfil, tema, status e rascunho usam sinais separados;
somente valores alterados são publicados. Digitar não remonta o histórico,
e as mensagens têm keys estáveis. Uma operação fica em andamento
por vez; os controles de escrita são desabilitados durante sua execução.
Ao fechar a janela, a operação pendente termina antes da thread ser encerrada.

Rascunhos usam uma fila coalescida por canal e debounce de 250 ms na thread do
banco. O compositor muda seu sinal local imediatamente; autosave não põe a UI
em estado ocupado nem reconstrói o histórico. Navegar/enviar força o flush e o
encerramento normal grava a última edição pendente. Falhas são recuperáveis e
aparecem no feedback; desligamento forçado/crash pode perder a janela de debounce.
Rascunhos preservam espaços e quebras de linha e aceitam até 65.536 caracteres
(o envio continua limitado a 4.000). Enviar mensagem e limpar seu rascunho é uma
única transação. Rascunhos de canais excluídos são removidos por cascata.

IDs são UUID v4 aleatórios, estáveis e sem significado temporal. Timestamps usam
milissegundos Unix UTC; o histórico é ordenado por `(created_at, id)` apenas para
exibição local determinística, sem promessa de ordenação global para P2P.
Nomes são aparados e limitados a 80 caracteres Unicode, sem controles; mensagens
são aparadas e limitadas a 4.000 caracteres, aceitam múltiplas linhas e rejeitam NUL.
Nomes de espaço são únicos no banco; nomes de canal são únicos dentro do espaço.
SQLite `NOCASE` ignora caixa apenas para ASCII: `Casa`/`casa` conflitam, enquanto
a comparação de caracteres acentuados não faz normalização Unicode.

Migrações SQL são embutidas e versionadas por `PRAGMA user_version`.
A **001 permanece byte a byte intacta**. A nova `002_local_ui.sql` acrescenta
apenas `channel_drafts` e `local_navigation`. Bancos v1 recebem somente essa
migração; instalações novas aplicam 001, perfil e 002 na mesma transação.
Erros fazem rollback; bancos com versão superior a 2 são recusados. Não há
conversão de mensagens, autores, UUIDs ou timestamps. Antes de atualizar, faça
backup com o aplicativo fechado. Uma futura fase de identidades deverá usar
a próxima versão livre (003 ou posterior), sem reaproveitar a 002 distribuída.
Chaves estrangeiras ficam habilitadas em toda conexão do adaptador. Exclusão de
espaço remove canais e mensagens; exclusão de canal remove suas mensagens.
As escritas usam parâmetros e as cascatas são atômicas no SQLite.
O timeout de bloqueio é de três segundos, fora da thread da UI.

### Limites desta entrega

O SQLite retorna páginas por `(created_at, id)` com ordenação descendente e
`LIMIT`, sem `OFFSET` ou leitura integral. Padrão: **50 mensagens**; limite
defensivo: **200** (mais uma linha de lookahead para `has_more`). Somente a página
é invertida para ordem cronológica. O índice composto da 001 já atende ao range
do cursor, como confirmado por `EXPLAIN QUERY PLAN` no teste de carga.
A API legada `messages()` é agora um helper limitado a 200; consumidores do
histórico completo devem iterar `messages_page()` e seu cursor.

**Mostrar anteriores** busca a próxima página mantendo a âncora visual.
A janela de aplicação/DOM fica limitada a **300 mensagens**; ao navegar para o
passado, as mensagens mais recentes que saírem dessa janela podem ser recuperadas
por **Voltar às recentes**. A deduplicação usa UUID. Novo texto acompanha o fim
somente se a leitura estava a até 72 px dele. A rolagem é imediata, sem animação
contínua, inclusive com reduced motion. Ainda não há virtualização completa de
linhas, busca, anexos, edição/exclusão individual ou sincronização entre instâncias.
O nome atual do perfil é usado para exibir a autoria das mensagens locais.
Tema, seleção de espaço/canal e rascunhos persistem. Posição exata da rolagem e
páginas carregadas são da sessão; reabrir começa na página recente do canal salvo.
O banco não é criptografado; proteja o diretório pelo sistema operacional e feche
o aplicativo antes de copiar o arquivo como backup. Exclusões não têm desfazer.
Não há garantias de desempenho: uma observação de processo debug não substitui
benchmark. Linux, acessibilidade completa e escalas de DPI variadas precisam de
validação em seus ambientes reais.

A próxima etapa sugerida é revisar a F1 e validar Linux/CI remota, DPI nativo e
leitor de tela. Fases G/H (identidades e rede) não foram implementadas e exigem
nova solicitação. Resultados de carga, abertura e memória debug/release estão
em `docs/VALIDATION.md`; amostras pontuais não demonstram a meta original de RAM.

## Licença

**A definir antes da primeira publicação.** Avaliar MIT/Apache-2.0 ou outra licença compatível com as dependências utilizadas. Não adicionar automaticamente uma licença sem decisão do mantenedor.

Dioxus 0.7.10 usa MIT/Apache-2.0. O Slint foi removido após a validação de paridade.
A licença do BoboCord continua pendente do mantenedor; nenhuma foi escolhida
automaticamente nesta implementação.
