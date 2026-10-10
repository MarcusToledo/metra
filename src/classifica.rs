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
