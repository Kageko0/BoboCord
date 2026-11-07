# BoboCord — Continuidade para o Codex (F → G → H)

**Base auditada:** `projeto(1).zip`, enviado em 07/11/2026. **Status:** Fase E (Slint → Dioxus) registrada como concluída; validações citadas abaixo são as registradas pelo agente no projeto, não reexecutadas nesta auditoria.

**Execução F1 no checkout:** os resultados atuais estão em
[docs/VALIDATION.md](docs/VALIDATION.md). Paginação SQL, estado local persistido,
ancoragem e testes de carga/regressão foram implementados. A CI foi encontrada e
confirmada no checkout; não se declara execução remota. A migração 001 foi
preservada e a 002 acrescenta somente estado local. As referências a problemas
e à numeração 002 para identidades abaixo representam a auditoria original:
a futura fase G deverá usar 003 ou posterior. G/H não foram executadas.

## Estado confirmado pelo código

- Workspace Rust Edition 2024: `bobocord-desktop`, `bobo-app`, `bobo-core`, `bobo-storage`.
- Dioxus 0.7.10 Desktop, Wry/WebView, CSS local modular, sinais separados, componentes shell/spaces/channels/chat/settings/modal.
- Domínio desacoplado, worker SQLite, CRUD de espaços/canais, perfil, tema, mensagens locais.
- O registro `docs/VALIDATION.md` relata 14 testes finais Rust, smoke nativo no WebView2 e build debug/release no Windows; Linux e CI remota não validados.
- O ZIP **não contém** `.github/workflows/ci.yml`, embora `docs/VALIDATION.md` declare que ele existe no checkout original. Verificar arquivo no repositório real; caso ausente, criar. Não declarar CI executada até que exista um run remoto.
- A função `Repository::messages(channel_id)` retorna todo o histórico, enquanto `bobo-app` recorta somente depois de obter todas as mensagens. **Não há paginação SQL real.**
- `messages.author_id` referencia `local_profile(id)`; autores externos exigem nova modelagem/migração.
- `MessageList` desenha todas as mensagens carregadas e executa `scrollIntoView(end)` sempre que `history` muda; ao carregar mensagens antigas, preservar posição de rolagem em vez de pular para baixo.
- Rascunhos são armazenados só em memória em `Application::drafts`; a seleção também não é restaurada após reinício.
- A interface atual mostra sempre o perfil local como autor; requer lookup por `author_id` após migração para múltiplos autores.

## Objetivo geral

Consolidar a interface Dioxus reativa, discreta e leve (navegação estilo Discord em blocos simples), concluir performance/persistência local e preparar um protocolo P2P que não dependa da VM Oracle para funcionar em uma rede direta. **Não incluir voz, vídeo, WebRTC, Oracle, bots ou captura de tela nesta entrega.** A VM Oracle E2.1.Micro é um serviço auxiliar futuro e não deve hospedar banco primário, conteúdo não criptografado ou mídia pesada.

## Fase F1 — Qualidade e eficiência local (primeira entrega obrigatória)

1. Baseline: registrar `git status`, hashes da migração 001, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo build --workspace --release --locked`; não sobrescrever dados existentes.
2. Garantir que `.github/workflows/ci.yml` existe no repositório efetivo. Adicionar Windows + Ubuntu se ausente, com dependências GTK3/WebKitGTK 4.1, fmt/clippy/test/build; não inventar resultado remoto.
3. Implementar `Repository::messages_page(channel_id, before: Option<MessageCursor>, limit: u32)` no contrato de domínio e no SQLite, usando paginação por chave `(created_at, id)` com `ORDER BY created_at DESC, id DESC LIMIT ?`; inverter somente o conjunto retornado para apresentação cronológica. Criar índice compatível e limite defensivo (ex.: 50 por página, máximo 200). Não utilizar `OFFSET` nem `SELECT` integral. Garantir cursor estável quando timestamps coincidirem e registros novos chegarem.
4. Alterar `bobo-app` para carregar a página inicial e recuperar anteriores sob demanda; retornar `has_more` e cursor em vez de tamanho total via leitura integral. Impedir duplicações; preservar rascunhos/canal ativo.
5. Corrigir autoscroll do chat: rolar para baixo ao enviar/receber nova mensagem *somente se a pessoa já estiver perto do fim*; ao buscar anteriores, preservar `scrollTop`/`scrollHeight` ou ancoragem equivalente. Respeitar reduced motion.
6. Persistir seleção de espaço/canal e rascunhos por canal em tabelas locais de preferências ou UI state. Guardar dados de texto de forma transacional, com gravação desacoplada/debounce curto e flush no encerramento; não bloquear o loop do Dioxus. Remover rascunhos de canais excluídos.
7. Refinar a interface Dioxus: estados hover/active/focus/disabled, tooltips, menu contextual leve, acessibilidade de formulários, placeholders/erros legíveis, estados vazios, atalhos. Não adicionar bibliotecas visuais grandes, CDN, polling de render ou re-render integral por tecla. CSS com tokens já existente.
8. Escrever testes: 10 mil mensagens distribuídas em canais, timestamps repetidos, paginação estável, deduplicação, isolamento de histórico, restauração de rascunho/seleção, carga anterior e posição do scroll no WebView real quando disponível.
9. Medir processo principal **e árvore WebView2** em idle e com histórico de 10 mil mensagens (com janela limitada). Documentar working set, memória privada, CPU e tempo de abertura separadamente. Não misturar métricas ou afirmar meta atendida sem evidência.

### Aceite F1

- Todas operações v0.1 anteriores continuam funcionando; nenhuma perda de banco existente.
- Consulta SQLite retorna apenas a página requisitada; interface não monta 10 mil elementos DOM simultaneamente.
- Histórico anterior não desloca a rolagem para o final; mensagens novas seguem comportamento previsto.
- Rascunhos/seleção sobrevivem ao reinício.
- `cargo fmt/clippy/test/build` passam nos ambientes efetivamente testados; resultados e limites relatados.

## Fase G — Modelo de identidades, eventos e segurança local (segunda entrega, após F1)

1. Definir `DeviceIdentity` e `UserIdentity` (chaves Ed25519 ou tecnologia equivalente aprovada), chave privada criada localmente por dispositivo e armazenada com permissões restritivas / armazenamento seguro do SO quando disponível. Nunca imprimir/telemetrar/exportar segredos. Definir explicitamente recuperação e troca de dispositivo; não confundir UUID do perfil com identidade criptográfica.
2. Criar migração `002_*` **transacional, idempotente quando aplicável** e preservando bancos v1. Não editar `001_foundation.sql`. Introduzir `users`/`members` (ou `identities`) e relacionar mensagens a autor local/remoto. Migrar o perfil atual para um registro correspondente, preservando UUIDs e histórico. Em exclusões de membro, preservar mensagens com atribuição segura. Proteger banco de downgrade e validar FK.
3. Exibir nome/avatar associados ao autor real da mensagem e fallback seguro para autor desconhecido. Não confiar em nome informado na rede para conceder permissão.
4. Definir eventos versionados em crate específico somente quando necessários: `event_id`, `space_id`, `channel_id`, `author/device`, `kind`, `payload`, `created_at`, contador ou sequência por origem, versão do protocolo, assinatura/autenticidade. Separar relógio de parede de ordenação causal. Validar tamanhos, formatos e limites antes de persistir.
5. Implementar armazenamento idempotente por `event_id`, deduplicação, fila outbound, receipts/acks e estado de sincronização. Definir políticas para edição/exclusão, conflito e ordenação determinística antes de ativar rede.
6. **Não implementar criptografia própria**. Selecionar primitivas/bibliotecas auditadas; threat model para pares maliciosos, convites, replay e vazamento de metadados. Documentar o que é ou não criptografado em repouso e em trânsito.
7. Testes de migração (v1 → v2), rollback de falha, compatibilidade, autores distintos, duplicação, evento malformado, invariantes de permissão e serialização.

### Aceite G

- DB v1 é migrado sem perda, autores locais e remotos coexistem, e mensagens antigas mantêm autoria.
- Eventos e sincronização possuem formato/versionamento e testes sem rede real.
- Identidades privadas não vazam em logs/repositório.

## Fase H — P2P funcional entre dois computadores (terceira entrega, após G)

1. Validar APIs e versões atuais do Iroh/QUIC e escolher uma versão fixa de início. Criar `bobo-network` sem acoplar Dioxus ao transporte.
2. Conexão manual com código de convite/identificador de endpoint, autenticação de identidade, confirmação do usuário e limites de entrada. Primeiro teste na LAN; depois entre redes separadas.
3. Troca de eventos de texto por canal, deduplicação, reenfileiramento após reinício e reconciliação idempotente. Enviar somente eventos autorizados para o espaço.
4. Reconexão com backoff, timeout, indicador de status real, cancelamento de tarefas, sem bloquear UI. Proibir aceitar comandos de banco arbitrários vindos da rede.
5. Teste com duas instâncias/bancos separados, simular duplicação, reordenação, perda e desconexão. Não declarar conexão P2P direta se o tráfego passou por relay.
6. Implantação opcional na Oracle *somente após* prova de conectividade: discovery/mailbox de mensagens cifradas e efêmeras, com TTL/quota/rate limit. Instância E2.1.Micro, 1GB; compilar backend fora dela; manter HTTPS e SSH restrito. Sem retransmissão pesada de áudio/vídeo.

### Aceite H

- Dois computadores trocam mensagens autenticadas dentro de um espaço e recuperam eventos perdidos após desconexão; testes descrevem topologia e se houve relay.
- Aplicativo permanece funcional offline e na ausência total da Oracle.

## Regras de execução do Codex

- Execute **apenas F1 neste ciclo**. Não começar G/H antes dos testes F1 e de registrar riscos.
- Atualize README, AGENTS, CODEX_NEXT_STEPS e docs/VALIDATION sem apagar histórico.
- Crie migrações novas, nunca altere a migração 001 distribuída.
- Sem push, deploy, alteração em recursos OCI ou dados reais sem pedido específico.
- Se não houver Linux, macOS ou CLI `dx`, registre precisamente o que não foi testado.
- Entregue lista de arquivos, comandos reais, resultados e métricas. Corrija primeiro regressões antes de novas features.

## Prompt para o Codex

> Leia integralmente README.md, AGENTS.md, CODEX_NEXT_STEPS.md, docs/VALIDATION.md e este arquivo. Audite o checkout real e execute somente a **Fase F1 — Qualidade e eficiência local**. Preserve a migração 001 e os bancos existentes. Implemente paginação SQL por cursor sem carregar histórico integral, scroll ancorado ao buscar mensagens anteriores, persistência de rascunhos e navegação, refinamento reativo e acessível do Dioxus, testes de regressão/carga e CI presente no repositório. Não implemente P2P, identidades remotas, Oracle, voz ou captura ainda. Rode os checks possíveis e documente resultados reais, limitações e próximos passos antes de iniciar a fase G.
