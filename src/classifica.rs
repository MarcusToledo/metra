//! Classificação pura das falhas do git a partir de stdout+stderr.

/// Motivo classificado de uma falha de pull/push.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TipoErroEnum {
    MergePendente,
    /// Arquivos extraídos das linhas "Merge conflict in <arquivo>".
    Conflito(Vec<String>),
    Divergente,
    AlteracoesLocais,
    ArquivosNaoRastreados,
    BranchRemotaInexistente,
    RemoteInexistente,
    PushRejeitado,
    RedeOuAutenticacao,
    HeadDestacado,
    /// Última linha não vazia da saída.
    Desconhecido(String),
}

/// Classifica a saída combinada de stdout e stderr pela primeira regra encontrada.
pub fn classificar(texto: &str) -> TipoErroEnum {
    if texto.contains("You have not concluded your merge")
        || texto.contains("Pulling is not possible because you have unmerged files")
    {
        return TipoErroEnum::MergePendente;
    }
    if texto.contains("CONFLICT (") {
        let mut arquivos = Vec::new();
        for linha in texto.lines() {
            if let Some((_, arquivo)) = linha.split_once("Merge conflict in ") {
                let arquivo = arquivo.trim();
                if !arquivos
                    .iter()
                    .any(|existente: &String| existente.as_str() == arquivo)
                {
                    arquivos.push(arquivo.to_string());
                }
            }
        }
        return TipoErroEnum::Conflito(arquivos);
    }
    if texto.contains("Need to specify how to reconcile divergent branches") {
        return TipoErroEnum::Divergente;
    }
    if texto.contains("Your local changes to the following files would be overwritten") {
        return TipoErroEnum::AlteracoesLocais;
    }
    if texto.contains("untracked working tree files would be overwritten") {
        return TipoErroEnum::ArquivosNaoRastreados;
    }
    if texto.contains("couldn't find remote ref") {
        return TipoErroEnum::BranchRemotaInexistente;
    }
    if texto.contains("does not appear to be a git repository") {
        return TipoErroEnum::RemoteInexistente;
    }
    if texto.contains("[rejected]") || texto.contains("Updates were rejected") {
        return TipoErroEnum::PushRejeitado;
    }
    if [
        "Permission denied",
        "Could not read from remote repository",
        "Could not resolve host",
        "Connection timed out",
        "Connection refused",
        "Host key verification failed",
        "terminal prompts disabled",
    ]
    .iter()
    .any(|mensagem| texto.contains(mensagem))
    {
        return TipoErroEnum::RedeOuAutenticacao;
    }

    TipoErroEnum::Desconhecido(
        texto
            .lines()
            .rev()
            .map(str::trim)
            .find(|linha| !linha.is_empty())
            .unwrap_or_default()
            .to_string(),
    )
}

impl TipoErroEnum {
    /// Texto curto exibido na lista de falhas.
    pub fn texto(&self) -> String {
        match self {
            Self::MergePendente => "merge pendente".to_string(),
            Self::Conflito(arquivos) => {
                if arquivos.is_empty() {
                    return "conflito".to_string();
                }
                let mut texto = format!(
                    "conflito em {}",
                    arquivos[..arquivos.len().min(3)].join(", ")
                );
                if arquivos.len() > 3 {
                    texto.push_str(&format!(" (+{})", arquivos.len() - 3));
                }
                texto
            }
            Self::Divergente => "branches divergentes".to_string(),
            Self::AlteracoesLocais => "alterações locais seriam sobrescritas".to_string(),
            Self::ArquivosNaoRastreados => {
                "arquivos não rastreados seriam sobrescritos".to_string()
            }
            Self::BranchRemotaInexistente => "branch não existe no remote".to_string(),
            Self::RemoteInexistente => "remote inexistente".to_string(),
            Self::PushRejeitado => "push rejeitado".to_string(),
            Self::RedeOuAutenticacao => "falha de rede ou autenticação".to_string(),
            Self::HeadDestacado => "HEAD destacado".to_string(),
            Self::Desconhecido(linha) => linha.clone(),
        }
    }

    /// Dica exibida no detalhe da falha.
    pub fn dica(&self) -> &'static str {
        match self {
            Self::Divergente => "use [n] para merge com --no-rebase",
            Self::Conflito(_)
            | Self::MergePendente
            | Self::AlteracoesLocais
            | Self::ArquivosNaoRastreados => "resolva manualmente: [p] abre pane no repo",
            _ => "veja o log",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TipoErroEnum, classificar};

    #[test]
    fn classifica_merge_pendente_com_arquivos_nao_mesclados() {
        let texto = "error: Pulling is not possible because you have unmerged files.\nhint: Fix them up in the work tree, and then use 'git add/rm <file>'\nhint: as appropriate to mark resolution and make a commit.\nfatal: Exiting because of an unresolved conflict.";
        assert_eq!(classificar(texto), TipoErroEnum::MergePendente);
    }

    #[test]
    fn classifica_merge_pendente_com_merge_head() {
        let texto = "fatal: You have not concluded your merge (MERGE_HEAD exists).\nPlease, commit your changes before you merge.";
        assert_eq!(classificar(texto), TipoErroEnum::MergePendente);
    }

    #[test]
    fn classifica_conflito_e_extrai_arquivo() {
        let texto = "Auto-merging a.txt\nCONFLICT (content): Merge conflict in a.txt\nAutomatic merge failed; fix conflicts and then commit the result.";
        assert_eq!(
            classificar(texto),
            TipoErroEnum::Conflito(vec!["a.txt".to_string()])
        );
    }

    #[test]
    fn classifica_conflito_com_arquivos_unicos_na_ordem() {
        let texto = "CONFLICT (content): Merge conflict in b.txt\nCONFLICT (content): Merge conflict in a.txt\nCONFLICT (content): Merge conflict in b.txt";
        assert_eq!(
            classificar(texto),
            TipoErroEnum::Conflito(vec!["b.txt".to_string(), "a.txt".to_string()])
        );
    }

    #[test]
    fn classifica_branches_divergentes() {
        let texto = "hint: You have divergent branches and need to specify how to reconcile them.\nhint: You can do so by running one of the following commands sometime before\nhint: your next pull:\nhint: \nhint:   git config pull.rebase false  # merge\nhint:   git config pull.rebase true   # rebase\nhint:   git config pull.ff only       # fast-forward only\nhint: \nhint: You can replace \"git config\" with \"git config --global\" to set a default\nhint: preference for all repositories. You can also pass --rebase, --no-rebase,\nhint: or --ff-only on the command line to override the configured default per\nhint: invocation.\nfatal: Need to specify how to reconcile divergent branches.";
        assert_eq!(classificar(texto), TipoErroEnum::Divergente);
    }

    #[test]
    fn classifica_alteracoes_locais() {
        let texto = "error: Your local changes to the following files would be overwritten by merge:\n\ta.txt\nPlease commit your changes or stash them before you merge.\nAborting";
        assert_eq!(classificar(texto), TipoErroEnum::AlteracoesLocais);
    }

    #[test]
    fn classifica_arquivos_nao_rastreados() {
        let texto = "error: The following untracked working tree files would be overwritten by merge:\n\tn.txt\nPlease move or remove them before you merge.\nAborting";
        assert_eq!(classificar(texto), TipoErroEnum::ArquivosNaoRastreados);
    }

    #[test]
    fn classifica_branch_remota_inexistente() {
        assert_eq!(
            classificar("fatal: couldn't find remote ref nao-existe"),
            TipoErroEnum::BranchRemotaInexistente
        );
    }

    #[test]
    fn classifica_remote_inexistente_antes_de_falha_de_autenticacao() {
        let texto = "fatal: 'upstream' does not appear to be a git repository\nfatal: Could not read from remote repository.\n\nPlease make sure you have the correct access rights\nand the repository exists.";
        assert_eq!(classificar(texto), TipoErroEnum::RemoteInexistente);
    }

    #[test]
    fn classifica_push_rejeitado() {
        let texto = " ! [rejected]        main -> main (fetch first)\nerror: failed to push some refs to '/tmp/x.git'\nhint: Updates were rejected because the remote contains work that you do";
        assert_eq!(classificar(texto), TipoErroEnum::PushRejeitado);
    }

    #[test]
    fn classifica_push_rejeitado_apenas_com_hint() {
        assert_eq!(
            classificar("hint: Updates were rejected because the remote contains work"),
            TipoErroEnum::PushRejeitado
        );
    }

    #[test]
    fn classifica_rede_ou_autenticacao() {
        for texto in [
            "fatal: unable to access 'https://host-inexistente.invalid/x.git/': Could not resolve host: host-inexistente.invalid",
            "Host key verification failed.\nfatal: Could not read from remote repository.",
            "fatal: could not read Username for 'https://x': terminal prompts disabled",
            "git@x: Permission denied (publickey).",
            "fatal: Could not read from remote repository.",
            "fatal: Connection timed out",
            "fatal: Connection refused",
        ] {
            assert_eq!(classificar(texto), TipoErroEnum::RedeOuAutenticacao);
        }
    }

    #[test]
    fn classifica_desconhecido_pela_ultima_linha_nao_vazia() {
        assert_eq!(
            classificar("linha a\nerro estranho final\n\n  \n"),
            TipoErroEnum::Desconhecido("erro estranho final".to_string())
        );
    }

    #[test]
    fn resume_ate_tres_arquivos_do_conflito() {
        let arquivos = ["a", "b", "c", "d", "e"]
            .map(|arquivo| arquivo.to_string())
            .to_vec();
        assert_eq!(
            TipoErroEnum::Conflito(arquivos).texto(),
            "conflito em a, b, c (+2)"
        );
        assert_eq!(
            TipoErroEnum::Conflito(vec!["a.txt".to_string()]).texto(),
            "conflito em a.txt"
        );
        assert_eq!(TipoErroEnum::Conflito(vec![]).texto(), "conflito");
    }

    #[test]
    fn mostra_texto_curto_de_cada_tipo() {
        for (tipo, texto) in [
            (TipoErroEnum::MergePendente, "merge pendente"),
            (TipoErroEnum::Divergente, "branches divergentes"),
            (
                TipoErroEnum::AlteracoesLocais,
                "alterações locais seriam sobrescritas",
            ),
            (
                TipoErroEnum::ArquivosNaoRastreados,
                "arquivos não rastreados seriam sobrescritos",
            ),
            (
                TipoErroEnum::BranchRemotaInexistente,
                "branch não existe no remote",
            ),
            (TipoErroEnum::RemoteInexistente, "remote inexistente"),
            (TipoErroEnum::PushRejeitado, "push rejeitado"),
            (
                TipoErroEnum::RedeOuAutenticacao,
                "falha de rede ou autenticação",
            ),
            (TipoErroEnum::HeadDestacado, "HEAD destacado"),
            (
                TipoErroEnum::Desconhecido("erro estranho".to_string()),
                "erro estranho",
            ),
        ] {
            assert_eq!(tipo.texto(), texto);
        }
    }

    #[test]
    fn mostra_dica_para_branches_divergentes() {
        assert_eq!(
            TipoErroEnum::Divergente.dica(),
            "use [n] para merge com --no-rebase"
        );
    }

    #[test]
    fn mostra_dica_para_resolucao_manual() {
        for tipo in [
            TipoErroEnum::Conflito(vec![]),
            TipoErroEnum::MergePendente,
            TipoErroEnum::AlteracoesLocais,
            TipoErroEnum::ArquivosNaoRastreados,
        ] {
            assert_eq!(tipo.dica(), "resolva manualmente: [p] abre pane no repo");
        }
    }

    #[test]
    fn mostra_dica_para_consultar_log() {
        for tipo in [
            TipoErroEnum::BranchRemotaInexistente,
            TipoErroEnum::RemoteInexistente,
            TipoErroEnum::PushRejeitado,
            TipoErroEnum::RedeOuAutenticacao,
            TipoErroEnum::HeadDestacado,
            TipoErroEnum::Desconhecido("erro".to_string()),
        ] {
            assert_eq!(tipo.dica(), "veja o log");
        }
    }
}
