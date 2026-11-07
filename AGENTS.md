# AGENTS.md — Instruções de desenvolvimento do BoboCord

Este arquivo orienta o Codex e outros agentes que modificarem este repositório. A v0.1 Foundation foi implementada e seu frontend passou pela **Fase E: Slint → Dioxus Desktop**. Leia `README.md`, `docs/VALIDATION.md` e `CODEX_NEXT_STEPS.md` antes de editar código. Execute somente a fase explicitamente solicitada; não avance automaticamente para rede ou mídia.

## 1. Missão

Manter uma fundação desktop modular e funcional em Rust, com interface Dioxus Desktop (RSX/CSS) e persistência SQLite, preparada para expandir para comunicação P2P e multimídia sem reescrever o domínio.

**Não** construir agora um clone completo do Discord. **Não** introduzir servidores obrigatórios. **Não** criar funcionalidades falsas apresentadas como prontas.

## 2. Escopo obrigatório da primeira entrega

1. Inicializar Cargo Workspace organizado, com binário `bobocord-desktop` e bibliotecas mínimas (`bobo-core`, `bobo-storage`, `bobo-protocol` apenas se houver uso real).
2. Criar UI desktop com: sidebar de espaços, lista de canais, área de mensagens, painel de perfil e tela de configurações.
3. Implementar ações **reais**: criar/renomear/excluir espaços, criar/renomear/excluir canais, escrever e listar mensagens locais, trocar tema.
4. Usar SQLite com migrações versionadas para armazenar espaços, canais, mensagens e preferências.
5. Garantir que cada ação de UI chame uma operação do domínio, e que a UI reflita o estado persistido; nunca codificar dados permanentes diretamente no RSX/CSS.
6. Adicionar testes unitários de regras do domínio e testes de integração de persistência com banco temporário.
7. Adicionar comandos de build/teste verificáveis e GitHub Actions básica (formatação, lint, testes).
8. Documentar no README como executar em Windows e Linux, com dependências confirmadas ou ressalvas explícitas quando não testadas.

## 3. Fora do escopo da v0.1

- P2P em produção, relay, NAT traversal e contas em nuvem.
- WebRTC, chamadas, Opus, RNNoise, câmera e compartilhamento de tela.
- Replay buffer, gravação de áudio/vídeo e atalhos globais.
- Integrações Spotify/YouTube, bots, plugins e Android.
- Criptografia ponta a ponta, convites distribuídos e permissões remotas.

É aceitável definir **interfaces pequenas** para evolução futura quando houver justificativa concreta; não criar implementações vazias ou dependências pesadas apenas por previsão.

## 4. Regras técnicas

- **Rust estável:** usar edição estável suportada; justificar versões no projeto.
- **Dioxus Desktop 0.7.10:** componentes RSX reutilizáveis, sinais com escopo adequado e CSS por tokens; manter regras de negócio fora da UI. Usar WebView2/WebKitGTK. Não reintroduzir Slint ou escolher Blitz experimental sem uma nova decisão explícita.
- **Renderização:** não inicializar WGPU/WGSL nem adicionar renderizadores alternativos por previsão. Empacotar CSS/assets locais, sem CDN ou comunicação externa silenciosa.
- **SQLite:** migrações determinísticas; chaves únicas; integridade referencial; transações para operações compostas; consultas parametrizadas.
- **Assíncrono:** não criar runtimes Tokio na aplicação por conveniência. Dioxus Desktop 0.7.10 exige seu runtime interno (confirmado por compilação e abertura); isso não justifica levar Tokio ao domínio/SQLite. Manter worker dedicado, respostas aguardadas por canal e nenhum polling na UI.
- **Dependências:** priorizar poucas crates mantidas, com licenças compatíveis; não adicionar bibliotecas futuras antes da necessidade.
- **Compatibilidade:** usar `std::path`, diretórios por usuário (sem caminhos hardcoded de Windows), separação clara entre I/O e domínio.
- **WebP:** permitido para assets; oferecer placeholders quando não houver imagens.
- **Segurança:** não registrar segredos nem credenciais; validar entradas; nenhuma comunicação externa silenciosa.

## 5. Modelo inicial de domínio

Entidades mínimas:

- `Space { id, name, created_at }`
- `Channel { id, space_id, name, kind, created_at }` — nesta fase somente `Text` deve ser operacional.
- `Message { id, channel_id, author_id, body, created_at }`
- `LocalProfile { id, display_name, avatar_path? }`
- `Preferences { theme, ... }`

Requisitos:

- Preferir IDs opacos/estáveis (UUID/ULID ou equivalente, com escolha documentada).
- Persistir timestamps de maneira consistente (UTC).
- Validar comprimentos e nomes vazios; exibir erros recuperáveis ao usuário.
- Deletar espaços/canais com comportamento de cascata explícito e testado.
- Evitar assumir ordenação global perfeita de mensagens para futura sincronização P2P.

## 6. Contratos e fronteiras

Organização desejada:

```text
UI (Dioxus Desktop / RSX / CSS)
  -> ações tipadas / aplicação (bobo-app, worker SQLite)
    -> comandos do domínio (bobo-core)
      -> repositórios / persistência (bobo-storage)
```

- O domínio não importa Dioxus ou implementações de banco.
- A UI não executa SQL.
- O armazenamento não depende de controles visuais.
- Separar erros de validação e erros de I/O para mensagens claras.
- Evitar `unwrap()` e `expect()` em caminhos comuns de produção; usá-los nos testes quando pertinente.
- Não executar bloqueios longos na thread principal.

## 7. Sequência de implementação (A–D: histórico da Foundation)

### Fase A — Bootstrap

- Inspecionar repositório existente; não sobrescrever trabalho do usuário.
- Criar workspace mínimo, configurar dependências e iniciar o binário.
- Implementar janela inicial com tema consistente e placeholders honestos.
- Validar compilação local; corrigir os erros antes de avançar.

### Fase B — Domínio e dados

- Criar entidades e regras de validação.
- Criar SQLite e migrações versionadas.
- Adicionar CRUD para espaços e canais e envio local de mensagens.
- Testar persistência e integridade.

### Fase C — Interface funcional

- Ligar ações da UI ao domínio.
- Carregar lista de espaços/canais/mensagens e atualizar a UI após operações.
- Implementar tema claro/escuro ou dois temas configuráveis.
- Incluir estados vazios, confirmações de exclusão e feedback de erro.

### Fase D — Qualidade e documentação

- Verificar formatação, lint e testes.
- Revisar consumo básico de memória e event loops; documentar medições se realizadas.
- Adicionar CI e instruções reais de execução.
- Produzir resumo do que funciona, limitações e próximos passos de v0.2.

### Fase E — Migração controlada do frontend

- Registrar baseline e preservar domínio, schema, banco existente e testes.
- Usar `bobo-app` para ações tipadas, seleção, rascunhos e worker, sem dependência de renderer.
- Criar componentes `AppShell`, `SpaceRail`, `ChannelSidebar`, `ChatHeader`, `MessageList`, `MessageComposer`, `ProfileDock`, `SettingsPanel` e `ModalHost`.
- Preferir sinais separados; digitar não deve remontar o histórico. Limitar o volume entregue à UI e manter SQL fora do loop gráfico.
- Usar CSS tokenizado com estados de foco/hover/seleção/disabled e reduced motion; modais precisam de confirmação e teclado funcional.
- Manter os frontends lado a lado até compilar, abrir e passar paridade equivalente. Remover o frontend antigo somente depois dessa evidência.
- Não alterar a migração 001 para trocar UI. Testes de domínio/SQLite são preservados; testes específicos do renderer são portados com cobertura equivalente ou maior.
- Manter `.github/workflows/ci.yml` com formatação, lint, testes e build em Windows/Ubuntu, incluindo os pacotes WebKitGTK necessários.
- Medir debug/release separadamente quando possível, incluindo processos WebView; registrar limites e verificações ausentes.

### Fase F1 — Qualidade, persistência e desempenho local

- Escopo detalhado em `BOBOCORD_CODEX_FGH.md`; não implementar G/H neste ciclo.
- Não editar a 001 distribuída. A 002 é de rascunhos/navegação local; futuras
  identidades deverão usar a próxima migração livre, sem reaproveitá-la.
- Usar `messages_page(channel, before, limit)` por chave `(created_at,id)`;
  padrão 50, máximo 200 mais lookahead, sem OFFSET ou leitura integral.
- Manter até 300 mensagens no cache/DOM e deduplicar por ID; preservar âncora
  ao carregar anteriores e só seguir novas mensagens quando perto do fim.
- Persistir navegação e rascunhos com transações, debounce no worker e flush
  no encerramento normal. Mensagem enviada e remoção do rascunho são atômicas.
- Não criar polling, timers de render ou escrita SQL por tecla na UI.
- Cobrir migração v1, rollback, cascatas, 10 mil mensagens com timestamps iguais,
  isolamento, reinício, rajadas de eventos, janela limitada e scroll nativo.
- Medir processo principal e árvore WebView separadamente, em debug/release e
  vazio/carga. Não confundir bytes privados, working set ou tempo com probe.
- Confirmar `.github/workflows/ci.yml` no checkout; CI remota só foi validada
  quando um run real tiver sido consultado. Não inferir Linux dos testes Windows.

As fases G/H e outros refinamentos não solicitados exigem nova instrução.

## 8. Definição de pronto

A v0.1 está pronta somente se:

- [ ] O app compila e abre no ambiente de execução disponível.
- [ ] Espaços e canais podem ser criados, renomeados e excluídos via UI.
- [ ] Mensagens locais aparecem no canal certo, persistem ao reiniciar e apresentam timestamps.
- [ ] Preferências de tema persistem.
- [ ] O sistema funciona sem Internet e sem Oracle.
- [ ] Banco e domínio têm testes automatizados relevantes.
- [ ] `cargo fmt --all -- --check` passa.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passa.
- [ ] `cargo test --workspace` passa.
- [ ] README descreve comandos testados e limitações reais.

Se um item não for verificável no ambiente (por exemplo, build em outro SO), registrar explicitamente **não verificado**, com comandos propostos para validação; não declarar sucesso sem executar.

## 9. Regras para mudanças do agente

- Priorizar alterações pequenas, compiláveis e revisáveis.
- Não trocar a stack, criar serviços em nuvem ou adicionar dependências extensas sem explicar a necessidade.
- Não criar mock de rede ou botões inertes sem indicar claramente o estado da função.
- Não remover testes para resolver erros de compilação.
- Sempre apresentar, ao concluir uma etapa: arquivos alterados, decisões tomadas, comandos executados, resultados, pendências e próxima ação sugerida.
- Manter compatibilidade com Windows/Linux; evitar APIs exclusivas de um deles na camada compartilhada.
- Não realizar push, publicação de releases, upload de arquivos ou implantação na Oracle sem solicitação explícita.

## 10. Próxima etapa após a v0.1 (não implementar agora)

Na v0.2, elaborar um `docs/PROTOCOL.md` com identidade por dispositivo, autenticação, descoberta, mensagens versionadas, deduplicação, confirmação de entrega, criptografia, regras de autorização e reconciliação offline. Antes de adotar Iroh, demonstrar conexão real entre dois dispositivos e cenários de NAT/relay. Só então integrar a rede ao domínio existente.

## 11. Prompt de início para o Codex

> Leia `README.md`, `AGENTS.md`, `docs/VALIDATION.md` e `CODEX_NEXT_STEPS.md` integralmente. Inspecione o estado atual e execute somente a fase solicitada. Preserve o domínio, SQLite e os testes existentes. Use ações tipadas e componentes Dioxus Desktop com CSS tokenizado. Valide formatação, Clippy, testes e build; não declare abertura, compatibilidade Linux ou desempenho sem executar a verificação. Registre arquivos alterados, decisões, comandos, resultados e limitações. Não faça push ou deploy, nem implemente fases posteriores sem solicitação.
