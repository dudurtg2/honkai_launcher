# Honkai Launcher

Launcher do Honkai Client em Rust: login com conta Microsoft, download do Minecraft 1.21.8
e do Java direto dos servidores da Mojang, e botão **Jogar**.

Ainda **não** tem agente, animações nem cosméticos. Isso vem nas próximas fases do planejamento.

## Usar agora

1. Abra `honkai_launcher.exe` (esta pasta).
2. Clique em **Entrar com Microsoft**, abra o link, digite o código e entre com a conta que tem o Minecraft.
3. Clique em **JOGAR**. Na primeira vez ele baixa cerca de 650 MB.

O ID do aplicativo Azure do Honkai (`37d65a5a-ba0d-423e-a2ee-fb1dd98540c9`) já vem dentro do launcher,
em `crates/launcher/src/configuracao.rs`. Os jogadores não precisam configurar nada.

Tudo fica em `%APPDATA%\HonkaiClient\` (separado do `.minecraft` oficial).
O login fica salvo no **Gerenciador de Credenciais do Windows**, nunca em arquivo de texto.

## Criar o aplicativo Azure (já feito; referência)

Sem isso o login não funciona. Todo launcher novo precisa do próprio.

1. Entre em <https://portal.azure.com> → **Microsoft Entra ID** → **Registros de aplicativo** → **Novo registro**.
2. Nome: `Honkai Client`. Tipo de conta: **Somente contas pessoais da Microsoft**. Não precisa de URI de redirecionamento.
3. No app criado → **Autenticação** → ative **Permitir fluxos de clientes públicos** → Salvar.
4. Copie o **ID do aplicativo (cliente)** da página **Visão geral**. É ele que vai no launcher.
5. Peça à Mojang acesso à API do Minecraft pelo formulário <https://aka.ms/mce-reviewappid>
   (informe o ID do aplicativo e o ID do diretório/locatário). A resposta pode levar semanas.

Até a Mojang aprovar, o login para na última etapa com **HTTP 403** e o launcher mostra essa mensagem.

## Versão de teste offline (`honkai_launcher_offline.exe`)

Para jogar e testar **sem conta Microsoft e sem o ID do Azure**: abra `honkai_launcher_offline.exe`,
digite um nome de jogador (3 a 16 letras, números ou _) e clique em **Entrar offline**.

- Funciona em mundo local e em servidores com `online-mode=false` (ex.: um servidor de teste seu).
- **Não use para distribuir aos jogadores.** O launcher oficial é o `honkai_launcher.exe`, sem modo offline.
- Usa a mesma pasta `%APPDATA%\HonkaiClient\`, então o que for baixado serve para as duas versões.

Gerar de novo:

```powershell
cargo build --release -p launcher --features modo_de_teste_offline
# copie target\release\honkai_launcher.exe como honkai_launcher_offline.exe
```

O `cargo run -p launcher` (desenvolvimento) também mostra o modo offline.

## Compilar

Precisa do Rust (<https://rustup.rs>). No Windows, o instalador do rustup pede as
"Visual Studio Build Tools"; aceite.

```powershell
cargo build --release -p launcher
# resultado: target\release\honkai_launcher.exe
cargo test
```

Teste de instalação e início sem interface (útil para depurar):

```powershell
cargo run --release -p inicializador_do_jogo --example testar_instalacao_e_inicio -- C:\temp\honkai_teste
```

## Onde fica cada coisa

```text
honkai_launcher/
├─ Cargo.toml                         workspace com os 4 crates
├─ honkai_launcher.exe                launcher pronto para Windows
└─ crates/
   ├─ autenticacao/                   login Microsoft → Xbox → Minecraft
   │  ├─ login_microsoft.rs           cada etapa do login, com mensagens de erro em português
   │  └─ cofre_de_credenciais.rs      salva o login no Gerenciador de Credenciais
   ├─ instalador_de_versoes/          baixa e confere os arquivos da Mojang
   │  ├─ versoes_validadas.rs         versões que o launcher aceita (hoje: 1.21.8)
   │  ├─ pastas.rs                    onde cada arquivo fica no disco
   │  ├─ modelos_da_mojang.rs         formato dos JSON da Mojang
   │  ├─ regras.rs                    regras de sistema das bibliotecas
   │  ├─ baixador.rs                  download paralelo com SHA-1
   │  ├─ runtime_java.rs              Java 21 da Mojang
   │  └─ instalacao.rs                junta tudo: instalar_versao
   ├─ inicializador_do_jogo/          monta o comando do Java e inicia o Minecraft
   └─ launcher/                       janela (egui)
      ├─ aplicativo.rs                estado da janela e ações do jogador
      ├─ telas/                       tela de login e tela principal com as 7 abas
      ├─ tarefas_em_segundo_plano.rs  login e downloads fora da janela
      ├─ configuracao.rs              configuracao.json
      └─ tema_visual.rs               cores do tema RPG
```

## Problemas comuns

| Mensagem | O que fazer |
| --- | --- |
| HTTP 403 ao entrar | A Mojang ainda não aprovou o app Azure (passo 5). |
| "não tem perfil Xbox" | Entre uma vez em minecraft.net com essa conta. |
| "não possui Minecraft: Java Edition" | A conta não comprou o jogo ou ainda não criou um nome de jogador. |
| O jogo fecha logo ao abrir | Configurações → **Abrir registro do último início**. Depois **Reparar arquivos e jogar**. |
