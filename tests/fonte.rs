use std::fs;
use std::path::Path;

use metra::fonte::{FonteEnum, descobrir, resolver_fonte};
use metra::repo::{SituacaoRepoEnum, situacao};
use tempfile::TempDir;

fn criar_git(caminho: &Path, como_arquivo: bool) {
    fs::create_dir_all(caminho).unwrap();
    if como_arquivo {
        fs::write(caminho.join(".git"), "gitdir: /tmp/exemplo\n").unwrap();
    } else {
        fs::create_dir(caminho.join(".git")).unwrap();
    }
}

#[test]
fn classifica_caminhos_pela_presenca_de_git_proprio() {
    let temporario = TempDir::new().unwrap();
    let diretorio = temporario.path().join("diretorio");
    let arquivo = temporario.path().join("arquivo");
    let sem_repositorio = temporario.path().join("sem-repositorio");
    criar_git(&diretorio, false);
    criar_git(&arquivo, true);
    fs::create_dir(&sem_repositorio).unwrap();

    assert_eq!(situacao(&diretorio), SituacaoRepoEnum::Clonado);
    assert_eq!(situacao(&arquivo), SituacaoRepoEnum::Clonado);
    assert_eq!(
        situacao(&temporario.path().join("ausente")),
        SituacaoRepoEnum::NaoClonado
    );
    assert_eq!(situacao(&sem_repositorio), SituacaoRepoEnum::SemRepositorio);
}

#[test]
fn meta_preserva_ordem_e_classifica_cada_caminho() {
    let temporario = TempDir::new().unwrap();
    let raiz = temporario.path();
    criar_git(&raiz.join("zeta"), false);
    fs::create_dir(raiz.join("grupo")).unwrap();
    fs::write(
        raiz.join(".meta"),
        r#"{"outro_campo": true, "projects": {"zeta": "u", "alfa": "u", "grupo/repo": "u"}}"#,
    )
    .unwrap();

    let (raiz_encontrada, repos) = descobrir(FonteEnum::Meta, raiz).unwrap();
    assert_eq!(raiz_encontrada, raiz);
    assert_eq!(
        repos
            .iter()
            .map(|repo| repo.nome.as_str())
            .collect::<Vec<_>>(),
        ["zeta", "alfa", "grupo/repo"]
    );
    assert_eq!(repos[0].caminho, raiz.join("zeta"));
    assert_eq!(repos[0].situacao, SituacaoRepoEnum::Clonado);
    assert_eq!(repos[1].caminho, raiz.join("alfa"));
    assert_eq!(repos[1].situacao, SituacaoRepoEnum::NaoClonado);
    assert_eq!(repos[2].caminho, raiz.join("grupo/repo"));
    assert_eq!(repos[2].situacao, SituacaoRepoEnum::NaoClonado);
}

#[test]
fn meta_classifica_pasta_sem_repositorio() {
    let temporario = TempDir::new().unwrap();
    let raiz = temporario.path();
    fs::create_dir(raiz.join("so-build")).unwrap();
    fs::write(raiz.join(".meta"), r#"{"projects":{"so-build":"u"}}"#).unwrap();

    let (_, repos) = descobrir(FonteEnum::Meta, raiz).unwrap();
    assert_eq!(repos[0].situacao, SituacaoRepoEnum::SemRepositorio);
}

#[test]
fn meta_invalido_inclui_caminho_e_erro_do_parser() {
    let temporario = TempDir::new().unwrap();
    let arquivo = temporario.path().join(".meta");
    fs::write(&arquivo, "{json invalido").unwrap();

    let erro = descobrir(FonteEnum::Meta, temporario.path()).unwrap_err();
    let mensagem = erro.to_string();
    assert!(
        mensagem.contains(&arquivo.display().to_string()),
        "{mensagem}"
    );
    assert!(mensagem.contains("key must be a string"), "{mensagem}");
}

#[test]
fn meta_sem_projects_retorna_erro_do_parser() {
    let temporario = TempDir::new().unwrap();
    let arquivo = temporario.path().join(".meta");
    fs::write(&arquivo, "{}").unwrap();

    let erro = descobrir(FonteEnum::Meta, temporario.path()).unwrap_err();
    let mensagem = erro.to_string();
    assert!(
        mensagem.contains(&arquivo.display().to_string()),
        "{mensagem}"
    );
    assert!(mensagem.contains("missing field `projects`"), "{mensagem}");
}

#[test]
fn meta_ausente_informa_diretorio_inicial() {
    let temporario = TempDir::new().unwrap();
    let inicio = temporario.path().join("sub");
    fs::create_dir(&inicio).unwrap();

    let erro = descobrir(FonteEnum::Meta, &inicio).unwrap_err();
    assert!(
        erro.to_string().contains(&format!(
            "nenhum .meta encontrado a partir de {}",
            inicio.display()
        )),
        "{erro}"
    );
}

#[test]
fn auto_encontra_meta_no_ancestral() {
    let temporario = TempDir::new().unwrap();
    let raiz = temporario.path();
    let inicio = raiz.join("sub/dir");
    fs::create_dir_all(&inicio).unwrap();
    fs::write(raiz.join(".meta"), r#"{"projects":{"repo":"u"}}"#).unwrap();

    let (raiz_encontrada, repos) = descobrir(FonteEnum::Auto, &inicio).unwrap();
    assert_eq!(raiz_encontrada, raiz);
    assert_eq!(repos[0].caminho, raiz.join("repo"));
    assert_eq!(repos[0].nome, "repo");
}

#[test]
fn varredura_ignora_ocultos_arquivos_pastas_sem_git_e_a_raiz() {
    let temporario = TempDir::new().unwrap();
    let raiz = temporario.path();
    criar_git(&raiz.join("b"), false);
    criar_git(&raiz.join("a"), true);
    criar_git(&raiz.join(".oculta"), false);
    fs::create_dir(raiz.join("sem")).unwrap();
    fs::create_dir(raiz.join(".git")).unwrap();
    fs::write(raiz.join("solto"), "conteudo").unwrap();

    let (raiz_encontrada, repos) = descobrir(FonteEnum::Varredura, raiz).unwrap();
    assert_eq!(raiz_encontrada, raiz);
    assert_eq!(
        repos
            .iter()
            .map(|repo| repo.nome.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(repos[0].caminho, raiz.join("a"));
    assert_eq!(repos[1].caminho, raiz.join("b"));
    assert!(
        repos
            .iter()
            .all(|repo| repo.situacao == SituacaoRepoEnum::Clonado)
    );
}

#[test]
fn auto_sem_meta_varre_o_diretorio_atual() {
    let temporario = TempDir::new().unwrap();
    let raiz = temporario.path();
    criar_git(&raiz.join("repo"), false);

    let (raiz_encontrada, repos) = descobrir(FonteEnum::Auto, raiz).unwrap();
    assert_eq!(raiz_encontrada, raiz);
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].caminho, raiz.join("repo"));
}

#[test]
fn varredura_ignora_meta_existente() {
    let temporario = TempDir::new().unwrap();
    let raiz = temporario.path();
    criar_git(&raiz.join("clonado"), false);
    fs::write(raiz.join(".meta"), r#"{"projects":{"ausente":"u"}}"#).unwrap();

    let (raiz_encontrada, fonte) = resolver_fonte(FonteEnum::Varredura, raiz).unwrap();
    let repos = fonte.listar().unwrap();
    assert_eq!(raiz_encontrada, raiz);
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].nome, "clonado");
}

#[test]
fn meta_vazio_nao_descobre_repositorios() {
    let temporario = TempDir::new().unwrap();
    let raiz = temporario.path();
    fs::write(raiz.join(".meta"), r#"{"projects":{}}"#).unwrap();

    let erro = descobrir(FonteEnum::Meta, raiz).unwrap_err();
    assert_eq!(
        erro.to_string(),
        format!("nenhum repositório encontrado em {}", raiz.display())
    );
}

#[test]
fn varredura_sem_repositorios_retorna_erro() {
    let temporario = TempDir::new().unwrap();
    let raiz = temporario.path();
    fs::create_dir(raiz.join("sem-git")).unwrap();

    let erro = descobrir(FonteEnum::Varredura, raiz).unwrap_err();
    assert_eq!(
        erro.to_string(),
        format!("nenhum repositório encontrado em {}", raiz.display())
    );
}
