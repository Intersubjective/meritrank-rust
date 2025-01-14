use crate::operations::*;
use crate::protocol::*;
use std::time::SystemTime;

fn put_testing_edges(graph: &mut AugMultiGraph) {
  graph.write_put_edge("", "U0cd6bd2dde4f", "B7f628ad203b5", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "C070e739180d6", 9.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "Bad1c69de7837", 7.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B92e4a185c654", 3.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B3c467fb437b2", -1.0, -1);
  graph.write_put_edge("", "U585dfead09c6", "C6d52e861b366", -1.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "C78d6fac93d00", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "Cbbf2df46955b", 1.0, -1);
  graph.write_put_edge("", "U4f530cfe771e", "B9c01ce5718d1", 0.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Cd6c9d5cba220", 1.0, -1);
  graph.write_put_edge("", "Cf4b448ef8618", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "Cbbf2df46955b", 4.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B73a44e2bbd44", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B5a1c1d3d0140", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U1df3e39ebe59", "Bea16f01b8cc5", 1.0, -1);
  graph.write_put_edge("", "Uadeb43da4abb", "Bfae1726e4e87", 1.0, -1);
  graph.write_put_edge("", "C599f6e6f6b64", "U26aca0e369c7", 1.0, -1);
  graph.write_put_edge("", "U79466f73dc0c", "B7f628ad203b5", 6.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B3c467fb437b2", -1.0, -1);
  graph.write_put_edge("", "Ud7002ae5a86c", "B75a44a52fa29", -2.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "C6acd550a4ef3", -1.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "B5eb4c6be535a", 5.0, -1);
  graph.write_put_edge("", "B9c01ce5718d1", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U99a0f1f7e6ee", "Bd90a1cf73384", 1.0, -1);
  graph.write_put_edge("", "U0e6659929c53", "Cffd169930956", 1.0, -1);
  graph.write_put_edge("", "Cd1c25e32ad21", "Ucd424ac24c15", 1.0, -1);
  graph.write_put_edge("", "Uac897fe92894", "B9c01ce5718d1", -2.0, -1);
  graph.write_put_edge("", "Bc4addf09b79f", "U0cd6bd2dde4f", 1.0, -1);
  graph.write_put_edge("", "U638f5c19326f", "B9cade9992fb9", 1.0, -1);
  graph.write_put_edge("", "U3c63a9b6115a", "Bad1c69de7837", 2.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "C6acd550a4ef3", 6.0, -1);
  graph.write_put_edge("", "U99a0f1f7e6ee", "C4d1d582c53c3", 1.0, -1);
  graph.write_put_edge("", "Be2b46c17f1da", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "B5e7178dd70bb", "Ucbd309d6fcc0", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "U1c285703fc63", -1.0, -1);
  graph.write_put_edge("", "C4893c40e481d", "Udece0afd9a8b", 1.0, -1);
  graph.write_put_edge("", "U9e42f6dab85a", "B3c467fb437b2", 1.0, -1);
  graph.write_put_edge("", "Ue70d59cc8e3f", "B9c01ce5718d1", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Bdf39d0e1daf5", -1.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "B70df5dbab8c3", 1.0, -1);
  graph.write_put_edge("", "Uad577360d968", "B5eb4c6be535a", 1.0, -1);
  graph.write_put_edge("", "U526f361717a8", "Cee9901f0f22c", 1.0, -1);
  graph.write_put_edge("", "C2bbd63b00224", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "Cb3c476a45037", "Ue40b938f47a4", 1.0, -1);
  graph.write_put_edge("", "C22e1102411ce", "U6661263fb410", 1.0, -1);
  graph.write_put_edge("", "U57b6f30fc663", "Bed5126bc655d", -1.0, -1);
  graph.write_put_edge("", "U6661263fb410", "Cf92f90725ffc", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "C2bbd63b00224", 8.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "Ba5d64165e5d5", -1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U79466f73dc0c", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B5eb4c6be535a", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B499bfc56e77b", -1.0, -1);
  graph.write_put_edge("", "U3c63a9b6115a", "Cf92f90725ffc", 1.0, -1);
  graph.write_put_edge("", "Ud04c89aaf453", "B4f14b223b56d", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Udece0afd9a8b", 1.0, -1);
  graph.write_put_edge("", "U38fdca6685ca", "Cf77494dc63d7", 1.0, -1);
  graph.write_put_edge("", "U83282a51b600", "Be2b46c17f1da", 0.0, -1);
  graph.write_put_edge("", "U83e829a2e822", "B7f628ad203b5", 14.0, -1);
  graph.write_put_edge("", "Bc896788cd2ef", "U1bcba4fd7175", 1.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "C67e4476fda28", 6.0, -1);
  graph.write_put_edge("", "C9028c7415403", "Udece0afd9a8b", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Uadeb43da4abb", "B0e230e9108dd", 4.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "C264c56d501db", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B73a44e2bbd44", 1.0, -1);
  graph.write_put_edge("", "Ud982a6dee46f", "Be7145faf15cb", 1.0, -1);
  graph.write_put_edge("", "B0a87a669fc28", "U34252014c05b", 1.0, -1);
  graph.write_put_edge("", "U0e6659929c53", "Cb967536095de", 1.0, -1);
  graph.write_put_edge("", "C0f834110f700", "U38fdca6685ca", 1.0, -1);
  graph.write_put_edge("", "U72f88cf28226", "Cb11edc3d0bc7", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "C0166be581dd4", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "C6a2263dc469e", 2.0, -1);
  graph.write_put_edge("", "U526f361717a8", "C52d41a9ad558", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Cb76829a425d9", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "Cf4b448ef8618", 1.0, -1);
  graph.write_put_edge("", "Uadeb43da4abb", "C30e7409c2d5f", 2.0, -1);
  graph.write_put_edge("", "U05e4396e2382", "B7f628ad203b5", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Cb11edc3d0bc7", 1.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "B1533941e2773", 1.0, -1);
  graph.write_put_edge("", "B506fff6cfc22", "Ub7f9dfb6a7a5", 1.0, -1);
  graph.write_put_edge("", "Uad577360d968", "C2bbd63b00224", 9.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "C4f2dafca724f", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Bd7a8bfcf3337", 1.0, -1);
  graph.write_put_edge("", "C1ccb4354d684", "Ue202d5b01f8d", 1.0, -1);
  graph.write_put_edge("", "Ud5b22ebf52f2", "Cd6c9d5cba220", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ba5d64165e5d5", -1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "C6aebafa4fe8e", 8.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "C588ffef22463", 1.0, -1);
  graph.write_put_edge("", "Ccae34b3da05e", "Ub93799d9400e", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B9c01ce5718d1", 3.0, -1);
  graph.write_put_edge("", "Uc35c445325f5", "B75a44a52fa29", 2.0, -1);
  graph.write_put_edge("", "U362d375c067c", "Ce06bda6030fe", 1.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "Cfdde53c79a2d", 3.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B75a44a52fa29", 1.0, -1);
  graph.write_put_edge("", "Bb5f87c1621d5", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "B3c467fb437b2", 2.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "Udece0afd9a8b", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B63fbe1427d09", -1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "C5782d559baad", 1.0, -1);
  graph.write_put_edge("", "C3b855f713d19", "U704bd6ecde75", 1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "Cb76829a425d9", 2.0, -1);
  graph.write_put_edge("", "U499f24158a40", "Ba3c4a280657d", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "Udece0afd9a8b", -1.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "Bdf39d0e1daf5", 1.0, -1);
  graph.write_put_edge("", "C588ffef22463", "Uef7fbf45ef11", 1.0, -1);
  graph.write_put_edge("", "U72f88cf28226", "B3f6f837bc345", 1.0, -1);
  graph.write_put_edge("", "Ba3c4a280657d", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Bd90a1cf73384", 3.0, -1);
  graph.write_put_edge("", "U638f5c19326f", "B9c01ce5718d1", 2.0, -1);
  graph.write_put_edge("", "U83282a51b600", "C9462ca240ceb", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "C54972a5fbc16", 1.0, -1);
  graph.write_put_edge("", "Ub93799d9400e", "B9c01ce5718d1", 5.0, -1);
  graph.write_put_edge("", "U9e42f6dab85a", "C15d8dfaceb75", 1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "Be2b46c17f1da", -1.0, -1);
  graph.write_put_edge("", "B8a531802473b", "U016217c34c6e", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "Bb78026d99388", -11.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "C4893c40e481d", 4.0, -1);
  graph.write_put_edge("", "Cb11edc3d0bc7", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Bb1e3630d2f4a", 1.0, -1);
  graph.write_put_edge("", "U0cd6bd2dde4f", "B92e4a185c654", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B45d72e29f004", -1.0, -1);
  graph.write_put_edge("", "Cab47a458295f", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ue55b928fa8dd", "Bed5126bc655d", 3.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "U9a89e0679dec", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "C6aebafa4fe8e", "U9a2c85753a6d", 1.0, -1);
  graph.write_put_edge("", "Ucdffb8ab5145", "Cf8fb8c05c116", 1.0, -1);
  graph.write_put_edge("", "U0cd6bd2dde4f", "B9c01ce5718d1", 1.0, -1);
  graph.write_put_edge("", "U59abf06369c3", "Cda989f4b466d", 1.0, -1);
  graph.write_put_edge("", "B4f00e7813add", "U09cf1f359454", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B75a44a52fa29", 3.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "U0c17798eaab4", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U09cf1f359454", 1.0, -1);
  graph.write_put_edge("", "U21769235b28d", "C801f204d0da8", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "B3c467fb437b2", 9.0, -1);
  graph.write_put_edge("", "U43dcf522b4dd", "B3b3f2ecde430", -1.0, -1);
  graph.write_put_edge("", "C264c56d501db", "U1bcba4fd7175", 1.0, -1);
  graph.write_put_edge("", "Ua4041a93bdf4", "B9c01ce5718d1", -1.0, -1);
  graph.write_put_edge("", "Uc3c31b8a022f", "B45d72e29f004", 3.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "C399b6349ab02", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "B3b3f2ecde430", 3.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B5eb4c6be535a", -1.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "Cfdde53c79a2d", 6.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "Uadeb43da4abb", -1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Bdf39d0e1daf5", -1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "Cbbf2df46955b", 5.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "C78ad459d3b81", 4.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B5a1c1d3d0140", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B25c85fe0df2d", -1.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "C6acd550a4ef3", 1.0, -1);
  graph.write_put_edge("", "B310b66ab31fb", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "C4b2b6fd8fa9a", 1.0, -1);
  graph.write_put_edge("", "B70df5dbab8c3", "U09cf1f359454", 1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "U09cf1f359454", 1.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "B75a44a52fa29", 1.0, -1);
  graph.write_put_edge("", "Uadeb43da4abb", "C9462ca240ceb", -1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "Bb78026d99388", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B491d307dfe01", 3.0, -1);
  graph.write_put_edge("", "C7c4d9ca4623e", "U8aa2e2623fa5", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "C1c86825bd597", 1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "C357396896bd0", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B4f00e7813add", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U9e42f6dab85a", 1.0, -1);
  graph.write_put_edge("", "U1e41b5f3adff", "B310b66ab31fb", 5.0, -1);
  graph.write_put_edge("", "Cc2b3069cbe5d", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "Uadeb43da4abb", 1.0, -1);
  graph.write_put_edge("", "U59abf06369c3", "B7f628ad203b5", 3.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "B45d72e29f004", -9.0, -1);
  graph.write_put_edge("", "U05e4396e2382", "Bad1c69de7837", -1.0, -1);
  graph.write_put_edge("", "Cd795a41fe71d", "U362d375c067c", 1.0, -1);
  graph.write_put_edge("", "U72f88cf28226", "B310b66ab31fb", 1.0, -1);
  graph.write_put_edge("", "B4f14b223b56d", "Ud04c89aaf453", 1.0, -1);
  graph.write_put_edge("", "U1e41b5f3adff", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U83e829a2e822", "B0e230e9108dd", -4.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "C6a2263dc469e", 3.0, -1);
  graph.write_put_edge("", "C89c123f7bcf5", "U8842ed397bb7", 1.0, -1);
  graph.write_put_edge("", "U26aca0e369c7", "C4893c40e481d", 2.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Uaa4e2be7a87a", -1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "C4893c40e481d", -1.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "B3f6f837bc345", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "C25639690ee57", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Ud9df8116deba", 1.0, -1);
  graph.write_put_edge("", "Ca8ceac412e6f", "U4ba2e4e81c0e", 1.0, -1);
  graph.write_put_edge("", "Be29b4af3f7a5", "Uc35c445325f5", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "U02fbd7c8df4c", 1.0, -1);
  graph.write_put_edge("", "Cb07d467c1c5e", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "U8aa2e2623fa5", "B9c01ce5718d1", -2.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B3b3f2ecde430", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Cf4b448ef8618", 2.0, -1);
  graph.write_put_edge("", "C9a2135edf7ff", "U83282a51b600", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B19ea554faf29", 1.0, -1);
  graph.write_put_edge("", "Ba5d64165e5d5", "U1e41b5f3adff", 1.0, -1);
  graph.write_put_edge("", "U9605bd4d1218", "B75a44a52fa29", 4.0, -1);
  graph.write_put_edge("", "B499bfc56e77b", "Uc1158424318a", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "Cd59e6cd7e104", 1.0, -1);
  graph.write_put_edge("", "U83e829a2e822", "Be2b46c17f1da", -8.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B45d72e29f004", -1.0, -1);
  graph.write_put_edge("", "Cb117f464e558", "U26aca0e369c7", 1.0, -1);
  graph.write_put_edge("", "U4ba2e4e81c0e", "B7f628ad203b5", -2.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "B19ea554faf29", 1.0, -1);
  graph.write_put_edge("", "Cfd59a206c07d", "U99a0f1f7e6ee", 1.0, -1);
  graph.write_put_edge("", "C8ece5c618ac1", "U21769235b28d", 1.0, -1);
  graph.write_put_edge("", "Uadeb43da4abb", "Cc9f863ff681b", 2.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "Cdcddfb230cb5", 5.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "Cc9f863ff681b", 1.0, -1);
  graph.write_put_edge("", "U26aca0e369c7", "C6acd550a4ef3", 4.0, -1);
  graph.write_put_edge("", "C8c753f46c014", "U8842ed397bb7", 1.0, -1);
  graph.write_put_edge("", "C78d6fac93d00", "Uc1158424318a", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "C357396896bd0", 8.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "Cd59e6cd7e104", 3.0, -1);
  graph.write_put_edge("", "Bf3a0a1165271", "U9a89e0679dec", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B70df5dbab8c3", 1.0, -1);
  graph.write_put_edge("", "Cb967536095de", "U0e6659929c53", 1.0, -1);
  graph.write_put_edge("", "C0b19d314485e", "Uaa4e2be7a87a", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "Bc896788cd2ef", -1.0, -1);
  graph.write_put_edge("", "Uc35c445325f5", "B9c01ce5718d1", 4.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "B9c01ce5718d1", 10.0, -1);
  graph.write_put_edge("", "C25639690ee57", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U362d375c067c", "Bad1c69de7837", 0.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "C67e4476fda28", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B60d725feca77", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Bfefe4e25c870", 3.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "Cdcddfb230cb5", 4.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "Cb14487d862b3", 1.0, -1);
  graph.write_put_edge("", "U682c3380036f", "C7986cd8a648a", 1.0, -1);
  graph.write_put_edge("", "U02fbd7c8df4c", "Bd7a8bfcf3337", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "Cbbf2df46955b", 5.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U6240251593cd", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "C8d80016b8292", 1.0, -1);
  graph.write_put_edge("", "Uc35c445325f5", "B8a531802473b", -5.0, -1);
  graph.write_put_edge("", "U704bd6ecde75", "B9c01ce5718d1", -1.0, -1);
  graph.write_put_edge("", "U77f496546efa", "B9c01ce5718d1", -1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B7f628ad203b5", -1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B10d3f548efc4", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "Cd06fea6a395f", 9.0, -1);
  graph.write_put_edge("", "U7cdd7999301e", "B7f628ad203b5", 1.0, -1);
  graph.write_put_edge("", "U526f361717a8", "Cf40e8fb326bc", 1.0, -1);
  graph.write_put_edge("", "B944097cdd968", "Ue40b938f47a4", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B3f6f837bc345", 1.0, -1);
  graph.write_put_edge("", "U6661263fb410", "Cc01e00342d63", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "Cb76829a425d9", -1.0, -1);
  graph.write_put_edge("", "Ccb7dc40f1513", "U6661263fb410", 1.0, -1);
  graph.write_put_edge("", "U83282a51b600", "C9a2135edf7ff", 1.0, -1);
  graph.write_put_edge("", "Cb76829a425d9", "Ue7a29d5409f2", 1.0, -1);
  graph.write_put_edge("", "B45d72e29f004", "U26aca0e369c7", 1.0, -1);
  graph.write_put_edge("", "Ue6cc7bfa0efd", "B5e7178dd70bb", -7.0, -1);
  graph.write_put_edge("", "Uac897fe92894", "Be2b46c17f1da", 2.0, -1);
  graph.write_put_edge("", "B73a44e2bbd44", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "C399b6349ab02", 5.0, -1);
  graph.write_put_edge("", "Cfa08a39f9bb9", "Ubebfe0c8fc29", 1.0, -1);
  graph.write_put_edge("", "Cdcddfb230cb5", "Udece0afd9a8b", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Bb5f87c1621d5", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "C78d6fac93d00", 2.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B3c467fb437b2", -1.0, -1);
  graph.write_put_edge("", "C0cd490b5fb6a", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "Be2b46c17f1da", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B499bfc56e77b", -1.0, -1);
  graph.write_put_edge("", "C2cb023b6bcef", "Ucb84c094edba", 1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "C4e0db8dec53e", 4.0, -1);
  graph.write_put_edge("", "Cc931cd2de143", "Ud7002ae5a86c", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "C4893c40e481d", 7.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U016217c34c6e", 1.0, -1);
  graph.write_put_edge("", "U682c3380036f", "U6240251593cd", 1.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "B4f00e7813add", 1.0, -1);
  graph.write_put_edge("", "Ud7002ae5a86c", "Cc931cd2de143", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "B79efabc4d8bf", 1.0, -1);
  graph.write_put_edge("", "U3de789cac826", "B9c01ce5718d1", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "C888c86d096d0", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "C10872dc9b863", 1.0, -1);
  graph.write_put_edge("", "B0e230e9108dd", "U9a89e0679dec", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "C2e31b4b1658f", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B25c85fe0df2d", -1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "C81f3f954b643", 1.0, -1);
  graph.write_put_edge("", "C0a576fc389d9", "U1bcba4fd7175", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B3f6f837bc345", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "C6d52e861b366", 3.0, -1);
  graph.write_put_edge("", "U362d375c067c", "Cd795a41fe71d", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B3b3f2ecde430", -1.0, -1);
  graph.write_put_edge("", "U585dfead09c6", "B9c01ce5718d1", 2.0, -1);
  graph.write_put_edge("", "Cfd47f43ac9cf", "U704bd6ecde75", 1.0, -1);
  graph.write_put_edge("", "U72f88cf28226", "Cd6c9d5cba220", 1.0, -1);
  graph.write_put_edge("", "Cdd49e516723a", "U704bd6ecde75", 1.0, -1);
  graph.write_put_edge("", "U26aca0e369c7", "Be2b46c17f1da", 7.0, -1);
  graph.write_put_edge("", "Uad577360d968", "C588ffef22463", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B3c467fb437b2", -1.0, -1);
  graph.write_put_edge("", "Bf34ee3bfc12b", "U6240251593cd", 1.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "Bb78026d99388", 9.0, -1);
  graph.write_put_edge("", "Ue202d5b01f8d", "B9c01ce5718d1", 2.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B310b66ab31fb", 1.0, -1);
  graph.write_put_edge("", "U35eb26fc07b4", "C90290100a953", 1.0, -1);
  graph.write_put_edge("", "Cc9f863ff681b", "Uc1158424318a", 1.0, -1);
  graph.write_put_edge("", "Uf5ee43a1b729", "C9218f86f6286", 1.0, -1);
  graph.write_put_edge("", "C888c86d096d0", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "Bfefe4e25c870", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "C6f84810d3cd9", 1.0, -1);
  graph.write_put_edge("", "Cd6c9d5cba220", "Ud5b22ebf52f2", 1.0, -1);
  graph.write_put_edge("", "U99a0f1f7e6ee", "C96bdee4f11e2", -18.0, -1);
  graph.write_put_edge("", "U4a82930ca419", "C2d9ab331aed7", 1.0, -1);
  graph.write_put_edge("", "C4818c4ed20bf", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ucd424ac24c15", "Cd1c25e32ad21", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "Bad1c69de7837", 2.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Cfdde53c79a2d", 5.0, -1);
  graph.write_put_edge("", "U5c827d7de115", "B69723edfec8a", 1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "Cd4417a5d718e", 5.0, -1);
  graph.write_put_edge("", "Ue202d5b01f8d", "C1ccb4354d684", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B9c01ce5718d1", 4.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "Cdcddfb230cb5", 1.0, -1);
  graph.write_put_edge("", "C81f3f954b643", "U09cf1f359454", 1.0, -1);
  graph.write_put_edge("", "U02fbd7c8df4c", "B75a44a52fa29", 7.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "C30e7409c2d5f", 4.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Be5bb2f3d56cb", -1.0, -1);
  graph.write_put_edge("", "B10d3f548efc4", "U99a0f1f7e6ee", 1.0, -1);
  graph.write_put_edge("", "Uc3c31b8a022f", "B3c467fb437b2", -1.0, -1);
  graph.write_put_edge("", "C90290100a953", "U35eb26fc07b4", 1.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "B310b66ab31fb", 1.0, -1);
  graph.write_put_edge("", "U35eb26fc07b4", "Be2b46c17f1da", 0.0, -1);
  graph.write_put_edge("", "Ccbd85b8513f3", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "Bc896788cd2ef", 1.0, -1);
  graph.write_put_edge("", "U0cd6bd2dde4f", "C7062e90f7422", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Be2b46c17f1da", -1.0, -1);
  graph.write_put_edge("", "C4d1d582c53c3", "U99a0f1f7e6ee", 1.0, -1);
  graph.write_put_edge("", "U59abf06369c3", "Cb117f464e558", -3.0, -1);
  graph.write_put_edge("", "B491d307dfe01", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "B25c85fe0df2d", "Uef7fbf45ef11", 1.0, -1);
  graph.write_put_edge("", "Bdf39d0e1daf5", "Uc1158424318a", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "C3e84102071d1", 6.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B63fbe1427d09", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Cd5983133fb67", 1.0, -1);
  graph.write_put_edge("", "Cc616eded7a99", "U0f63ee3db59b", 1.0, -1);
  graph.write_put_edge("", "U34252014c05b", "B19ea554faf29", 1.0, -1);
  graph.write_put_edge("", "U0f63ee3db59b", "B9c01ce5718d1", -4.0, -1);
  graph.write_put_edge("", "Uf6ce05bc4e5a", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Cbce32a9b256a", 1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "Bf3a0a1165271", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "B63fbe1427d09", -3.0, -1);
  graph.write_put_edge("", "U0cd6bd2dde4f", "Bc4addf09b79f", 1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "B4f00e7813add", 3.0, -1);
  graph.write_put_edge("", "U9e42f6dab85a", "Bad1c69de7837", 3.0, -1);
  graph.write_put_edge("", "U26aca0e369c7", "C599f6e6f6b64", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "Bdf39d0e1daf5", -1.0, -1);
  graph.write_put_edge("", "Uf8bf10852d43", "B253177f84f08", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "B7f628ad203b5", 1.0, -1);
  graph.write_put_edge("", "U43dcf522b4dd", "B9c01ce5718d1", 2.0, -1);
  graph.write_put_edge("", "C13e2a35d917a", "Uf6ce05bc4e5a", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B8a531802473b", -1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "C247501543b60", 1.0, -1);
  graph.write_put_edge("", "C2e31b4b1658f", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "C94bb73c10a06", "Uef7fbf45ef11", 1.0, -1);
  graph.write_put_edge("", "C357396896bd0", "Udece0afd9a8b", 1.0, -1);
  graph.write_put_edge("", "C6acd550a4ef3", "Uc1158424318a", 1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "C3e84102071d1", 1.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "Bc4addf09b79f", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "Cfe90cbd73eab", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "C30e7409c2d5f", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "Bf3a0a1165271", -1.0, -1);
  graph.write_put_edge("", "Uadeb43da4abb", "C2bbd63b00224", 7.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Be2b46c17f1da", -1.0, -1);
  graph.write_put_edge("", "Caa62fc21e191", "U4ba2e4e81c0e", 1.0, -1);
  graph.write_put_edge("", "U02fbd7c8df4c", "Bad1c69de7837", -5.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "B73a44e2bbd44", 3.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "U9e42f6dab85a", 1.0, -1);
  graph.write_put_edge("", "Cb95e21215efa", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "B1533941e2773", "U79466f73dc0c", 1.0, -1);
  graph.write_put_edge("", "U1e41b5f3adff", "Ba5d64165e5d5", 1.0, -1);
  graph.write_put_edge("", "U682c3380036f", "Bf34ee3bfc12b", 4.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "Uc3c31b8a022f", -1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "U9ce5721e93cf", "B68247950d9c0", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "Uc3c31b8a022f", 1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "Cd06fea6a395f", -1.0, -1);
  graph.write_put_edge("", "U9e42f6dab85a", "C6a2263dc469e", 5.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B0a87a669fc28", 1.0, -1);
  graph.write_put_edge("", "Cf40e8fb326bc", "U526f361717a8", 1.0, -1);
  graph.write_put_edge("", "U4ba2e4e81c0e", "Cb117f464e558", 1.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "B9c01ce5718d1", 2.0, -1);
  graph.write_put_edge("", "C524134905072", "Ucb84c094edba", 1.0, -1);
  graph.write_put_edge("", "Cd59e6cd7e104", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "Cbbf2df46955b", -1.0, -1);
  graph.write_put_edge("", "Cbe89905f07d3", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "Bed5126bc655d", "Uc4ebbce44401", 1.0, -1);
  graph.write_put_edge("", "U9605bd4d1218", "B8a531802473b", 2.0, -1);
  graph.write_put_edge("", "Ub93799d9400e", "B73a44e2bbd44", 5.0, -1);
  graph.write_put_edge("", "Cee9901f0f22c", "U526f361717a8", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Cc2b3069cbe5d", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "B3b3f2ecde430", 1.0, -1);
  graph.write_put_edge("", "Ubebfe0c8fc29", "Cfa08a39f9bb9", 1.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "Be7bc0cfecab3", 1.0, -1);
  graph.write_put_edge("", "C6587e913fbbe", "U6661263fb410", 1.0, -1);
  graph.write_put_edge("", "U0cd6bd2dde4f", "C5782d559baad", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B25c85fe0df2d", -1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "B8a531802473b", 1.0, -1);
  graph.write_put_edge("", "Ccc25a77bfa2a", "U77f496546efa", 1.0, -1);
  graph.write_put_edge("", "U6240251593cd", "Bf34ee3bfc12b", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "C357396896bd0", 1.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "Cb76829a425d9", 8.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "U016217c34c6e", 1.0, -1);
  graph.write_put_edge("", "Ucdffb8ab5145", "B9c01ce5718d1", 2.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "B75a44a52fa29", 1.0, -1);
  graph.write_put_edge("", "Cac6ca02355da", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Bc4addf09b79f", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "U389f9f24b31c", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ud9df8116deba", 1.0, -1);
  graph.write_put_edge("", "Ucd424ac24c15", "B9c01ce5718d1", 2.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "B9c01ce5718d1", 9.0, -1);
  graph.write_put_edge("", "U3c63a9b6115a", "B75a44a52fa29", 5.0, -1);
  graph.write_put_edge("", "Bea16f01b8cc5", "U1df3e39ebe59", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "C3fd1fdebe0e9", 9.0, -1);
  graph.write_put_edge("", "Cffd169930956", "U0e6659929c53", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "B3b3f2ecde430", -3.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "C4e0db8dec53e", 1.0, -1);
  graph.write_put_edge("", "U83282a51b600", "B9c01ce5718d1", -1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "U9e42f6dab85a", -1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Bfefe4e25c870", 4.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "C070e739180d6", 1.0, -1);
  graph.write_put_edge("", "C8343a6a576ff", "U02fbd7c8df4c", 1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "C599f6e6f6b64", 2.0, -1);
  graph.write_put_edge("", "U77f496546efa", "C9462ca240ceb", -1.0, -1);
  graph.write_put_edge("", "Cc42c3eeb9d20", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "Ce1a7d8996eb0", -1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "B70df5dbab8c3", 2.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "C35678a54ef5f", 1.0, -1);
  graph.write_put_edge("", "U72f88cf28226", "C7722465c957a", 1.0, -1);
  graph.write_put_edge("", "U9605bd4d1218", "C801f204d0da8", 3.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B10d3f548efc4", 3.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "Be7145faf15cb", "Ud982a6dee46f", 1.0, -1);
  graph.write_put_edge("", "Cd06fea6a395f", "Uaa4e2be7a87a", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "C78ad459d3b81", 6.0, -1);
  graph.write_put_edge("", "Bb1e3630d2f4a", "U34252014c05b", 1.0, -1);
  graph.write_put_edge("", "U6661263fb410", "B75a44a52fa29", 3.0, -1);
  graph.write_put_edge("", "Uadeb43da4abb", "Bd49e3dac97b0", 1.0, -1);
  graph.write_put_edge("", "Ub93799d9400e", "Cd4417a5d718e", 1.0, -1);
  graph.write_put_edge("", "C399b6349ab02", "Uf2b0a6b1d423", 1.0, -1);
  graph.write_put_edge("", "B79efabc4d8bf", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U83282a51b600", "C16dfdd8077c8", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U9a2c85753a6d", 1.0, -1);
  graph.write_put_edge("", "B19ea554faf29", "U34252014c05b", 1.0, -1);
  graph.write_put_edge("", "B75a44a52fa29", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "C35678a54ef5f", "Uaa4e2be7a87a", 1.0, -1);
  graph.write_put_edge("", "Bc173d5552e2e", "U95f3426b8e5d", 1.0, -1);
  graph.write_put_edge("", "Uc3c31b8a022f", "Bb78026d99388", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "Be5bb2f3d56cb", -1.0, -1);
  graph.write_put_edge("", "Cb62aea64ea97", "U0e6659929c53", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "B25c85fe0df2d", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B8a531802473b", -1.0, -1);
  graph.write_put_edge("", "C5127d08eb786", "Ucd424ac24c15", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "Be2b46c17f1da", 5.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "C6a2263dc469e", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "B0e230e9108dd", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B4f14b223b56d", -1.0, -1);
  graph.write_put_edge("", "B3c467fb437b2", "U9e42f6dab85a", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "C30fef1977b4a", 8.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B19ea554faf29", 3.0, -1);
  graph.write_put_edge("", "U6240251593cd", "B9c01ce5718d1", -4.0, -1);
  graph.write_put_edge("", "U99a0f1f7e6ee", "C1f41b842849c", 1.0, -1);
  graph.write_put_edge("", "Uac897fe92894", "Cb117f464e558", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "U0cd6bd2dde4f", 1.0, -1);
  graph.write_put_edge("", "Ucb84c094edba", "C524134905072", 1.0, -1);
  graph.write_put_edge("", "B19d70698e3d8", "Uf8bf10852d43", 1.0, -1);
  graph.write_put_edge("", "Cda989f4b466d", "U59abf06369c3", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B1533941e2773", 3.0, -1);
  graph.write_put_edge("", "U83e829a2e822", "B5eb4c6be535a", 3.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "Bd7a8bfcf3337", 3.0, -1);
  graph.write_put_edge("", "Cfdde53c79a2d", "Uef7fbf45ef11", 1.0, -1);
  graph.write_put_edge("", "Ue6cc7bfa0efd", "B30bf91bf5845", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B3b3f2ecde430", -1.0, -1);
  graph.write_put_edge("", "C63e21d051dda", "U638f5c19326f", 1.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "C30e7409c2d5f", 9.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "C9028c7415403", 3.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "Bd49e3dac97b0", -1.0, -1);
  graph.write_put_edge("", "C279db553a831", "U99a0f1f7e6ee", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B45d72e29f004", -1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "B491d307dfe01", -1.0, -1);
  graph.write_put_edge("", "U99a0f1f7e6ee", "Cfd59a206c07d", 1.0, -1);
  graph.write_put_edge("", "C52d41a9ad558", "U526f361717a8", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Uc3c31b8a022f", -1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "C0a576fc389d9", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "C94bb73c10a06", 3.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ud5b22ebf52f2", 1.0, -1);
  graph.write_put_edge("", "Uf8bf10852d43", "B4115d364e05b", 1.0, -1);
  graph.write_put_edge("", "U57b6f30fc663", "B30bf91bf5845", 1.0, -1);
  graph.write_put_edge("", "U72f88cf28226", "U6d2f25cc4264", 0.0, -1);
  graph.write_put_edge("", "U3c63a9b6115a", "Be5bb2f3d56cb", 1.0, -1);
  graph.write_put_edge("", "C7722465c957a", "U72f88cf28226", 1.0, -1);
  graph.write_put_edge("", "Ub7f9dfb6a7a5", "B506fff6cfc22", 1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "C9028c7415403", 1.0, -1);
  graph.write_put_edge("", "U79466f73dc0c", "B45d72e29f004", 5.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B3f6f837bc345", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B4f14b223b56d", -1.0, -1);
  graph.write_put_edge("", "U6661263fb410", "C31dac67e313b", 1.0, -1);
  graph.write_put_edge("", "C55a114ca6e7c", "U0e6659929c53", 1.0, -1);
  graph.write_put_edge("", "C4b2b6fd8fa9a", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U9e42f6dab85a", "C0b19d314485e", -1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Ba3c4a280657d", 2.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B70df5dbab8c3", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "C30fef1977b4a", 1.0, -1);
  graph.write_put_edge("", "Uc35c445325f5", "Be29b4af3f7a5", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "B5eb4c6be535a", 1.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "Cdcddfb230cb5", 3.0, -1);
  graph.write_put_edge("", "U9605bd4d1218", "Bd7a8bfcf3337", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "B9c01ce5718d1", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "B3b3f2ecde430", 9.0, -1);
  graph.write_put_edge("", "U83282a51b600", "B45d72e29f004", -1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B60d725feca77", -1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "Cd59e6cd7e104", 1.0, -1);
  graph.write_put_edge("", "U26aca0e369c7", "C9028c7415403", 8.0, -1);
  graph.write_put_edge("", "B9cade9992fb9", "U638f5c19326f", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B0e230e9108dd", -1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U79466f73dc0c", "Be2b46c17f1da", 4.0, -1);
  graph.write_put_edge("", "C9218f86f6286", "Uf5ee43a1b729", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Bb78026d99388", -1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "B3b3f2ecde430", 6.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Bd49e3dac97b0", -1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "C9462ca240ceb", 1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "B0e230e9108dd", -1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "Uf5096f6ab14e", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Uf2b0a6b1d423", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Bd90a1cf73384", 1.0, -1);
  graph.write_put_edge("", "Ucb84c094edba", "C2cb023b6bcef", 1.0, -1);
  graph.write_put_edge("", "Ubebfe0c8fc29", "Bfefe4e25c870", 3.0, -1);
  graph.write_put_edge("", "U9e42f6dab85a", "C070e739180d6", 2.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "C6f84810d3cd9", 1.0, -1);
  graph.write_put_edge("", "U14a3c81256ab", "B9c01ce5718d1", 0.0, -1);
  graph.write_put_edge("", "Cd5983133fb67", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "U0cd6bd2dde4f", "B75a44a52fa29", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Ce1a7d8996eb0", 5.0, -1);
  graph.write_put_edge("", "Ue40b938f47a4", "B9c01ce5718d1", 0.0, -1);
  graph.write_put_edge("", "U0e6659929c53", "Cb62aea64ea97", 1.0, -1);
  graph.write_put_edge("", "C1c86825bd597", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "Bb78026d99388", -1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "Cbce32a9b256a", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "C96bdee4f11e2", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "C4893c40e481d", 3.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "B7f628ad203b5", 7.0, -1);
  graph.write_put_edge("", "Uad577360d968", "Bad1c69de7837", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "Cdeab5b39cc2a", 1.0, -1);
  graph.write_put_edge("", "B7f628ad203b5", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "C4893c40e481d", 1.0, -1);
  graph.write_put_edge("", "C96bdee4f11e2", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B0e230e9108dd", -1.0, -1);
  graph.write_put_edge("", "U02fbd7c8df4c", "C8343a6a576ff", 1.0, -1);
  graph.write_put_edge("", "C30fef1977b4a", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "U77f496546efa", "Ccc25a77bfa2a", 1.0, -1);
  graph.write_put_edge("", "Uf6ce05bc4e5a", "C13e2a35d917a", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Cfe90cbd73eab", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "B3c467fb437b2", 2.0, -1);
  graph.write_put_edge("", "U9e42f6dab85a", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "Cfdde53c79a2d", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "C94bb73c10a06", 9.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Bb78026d99388", -1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B499bfc56e77b", -1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "Ce1a7d8996eb0", 6.0, -1);
  graph.write_put_edge("", "C7062e90f7422", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "Cf8fb8c05c116", "Ucdffb8ab5145", 1.0, -1);
  graph.write_put_edge("", "B60d725feca77", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "C070e739180d6", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "C3e84102071d1", "U016217c34c6e", 1.0, -1);
  graph.write_put_edge("", "B69723edfec8a", "U5c827d7de115", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "B0e230e9108dd", 3.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "C4e0db8dec53e", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "Cfc639b9aa3e0", 1.0, -1);
  graph.write_put_edge("", "Uad577360d968", "C0cd490b5fb6a", 1.0, -1);
  graph.write_put_edge("", "Uc3c31b8a022f", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "U3c63a9b6115a", "B9c01ce5718d1", 3.0, -1);
  graph.write_put_edge("", "B3b3f2ecde430", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "U38fdca6685ca", "B9c01ce5718d1", 0.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "C78ad459d3b81", 1.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "C67e4476fda28", -1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "U4ba2e4e81c0e", "Ca8ceac412e6f", 1.0, -1);
  graph.write_put_edge("", "Cfe90cbd73eab", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U79466f73dc0c", "B1533941e2773", 1.0, -1);
  graph.write_put_edge("", "C8d80016b8292", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Uf6ce05bc4e5a", "Bf843e315d71b", 1.0, -1);
  graph.write_put_edge("", "U6661263fb410", "C6587e913fbbe", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Bb1e3630d2f4a", 3.0, -1);
  graph.write_put_edge("", "Ucd424ac24c15", "C5127d08eb786", 1.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "C4e0db8dec53e", 4.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Bad1c69de7837", -1.0, -1);
  graph.write_put_edge("", "Ud7002ae5a86c", "C7a807e462b65", 1.0, -1);
  graph.write_put_edge("", "C3c17b70c3357", "U3de789cac826", 1.0, -1);
  graph.write_put_edge("", "Bd90a1cf73384", "U99a0f1f7e6ee", 1.0, -1);
  graph.write_put_edge("", "U83282a51b600", "B7f628ad203b5", 1.0, -1);
  graph.write_put_edge("", "U26aca0e369c7", "Cb117f464e558", 1.0, -1);
  graph.write_put_edge("", "Ce1a7d8996eb0", "Uf5096f6ab14e", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "Bad1c69de7837", -1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B25c85fe0df2d", -1.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "Bc173d5552e2e", 1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "C6aebafa4fe8e", 8.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "B8a531802473b", 8.0, -1);
  graph.write_put_edge("", "Ub93799d9400e", "B75a44a52fa29", 5.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Bf34ee3bfc12b", 1.0, -1);
  graph.write_put_edge("", "U34252014c05b", "B0a87a669fc28", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Cbe89905f07d3", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "Ce1a7d8996eb0", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B4f00e7813add", 1.0, -1);
  graph.write_put_edge("", "Bd7a8bfcf3337", "U02fbd7c8df4c", 1.0, -1);
  graph.write_put_edge("", "C2d9ab331aed7", "U4a82930ca419", 1.0, -1);
  graph.write_put_edge("", "C247501543b60", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "Cfdde53c79a2d", 4.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "C613f00c1333c", 1.0, -1);
  graph.write_put_edge("", "C31dac67e313b", "U6661263fb410", 1.0, -1);
  graph.write_put_edge("", "C67e4476fda28", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "C3e84102071d1", 4.0, -1);
  graph.write_put_edge("", "U9605bd4d1218", "Cab47a458295f", 3.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "C992d8370db6b", 1.0, -1);
  graph.write_put_edge("", "Cbce32a9b256a", "U389f9f24b31c", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B63fbe1427d09", -1.0, -1);
  graph.write_put_edge("", "B63fbe1427d09", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "B60d725feca77", 8.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "Bfefe4e25c870", 5.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B9c01ce5718d1", 2.0, -1);
  graph.write_put_edge("", "Uac897fe92894", "B7f628ad203b5", 1.0, -1);
  graph.write_put_edge("", "B8120aa1edccb", "Ue40b938f47a4", 1.0, -1);
  graph.write_put_edge("", "Ueb139752b907", "U79466f73dc0c", 1.0, -1);
  graph.write_put_edge("", "U0e6659929c53", "B9c01ce5718d1", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B491d307dfe01", 2.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "B73a44e2bbd44", 1.0, -1);
  graph.write_put_edge("", "U38fdca6685ca", "C958e7588ae1c", 1.0, -1);
  graph.write_put_edge("", "U35eb26fc07b4", "B60d725feca77", 1.0, -1);
  graph.write_put_edge("", "Cdeab5b39cc2a", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U0f63ee3db59b", "Cbcf72c7e6061", 1.0, -1);
  graph.write_put_edge("", "C4e0db8dec53e", "U0c17798eaab4", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B7f628ad203b5", -1.0, -1);
  graph.write_put_edge("", "C472b59eeafa5", "U4a82930ca419", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "C247501543b60", 1.0, -1);
  graph.write_put_edge("", "Cfc639b9aa3e0", "U389f9f24b31c", 1.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "B23b74174e659", 1.0, -1);
  graph.write_put_edge("", "U99deecf5a281", "B9c01ce5718d1", 1.0, -1);
  graph.write_put_edge("", "Bfefe4e25c870", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ua12e78308f49", "B75a44a52fa29", 4.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "Cd06fea6a395f", 1.0, -1);
  graph.write_put_edge("", "U682c3380036f", "B75a44a52fa29", 2.0, -1);
  graph.write_put_edge("", "Ue202d5b01f8d", "C637133747308", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Cab47a458295f", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "Cd06fea6a395f", 8.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Bd7a8bfcf3337", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "Uf5ee43a1b729", "B47cc49866c37", 1.0, -1);
  graph.write_put_edge("", "Uac897fe92894", "C9462ca240ceb", 0.0, -1);
  graph.write_put_edge("", "U21769235b28d", "C481cd737c873", 1.0, -1);
  graph.write_put_edge("", "C6f84810d3cd9", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Uc3c31b8a022f", "C78d6fac93d00", 3.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "C4f2dafca724f", 8.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "B0e230e9108dd", 2.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Bb78026d99388", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Cdcddfb230cb5", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U0cd6bd2dde4f", 1.0, -1);
  graph.write_put_edge("", "Ud5b22ebf52f2", "B310b66ab31fb", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B0e230e9108dd", -1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Uad577360d968", "Cbce32a9b256a", 3.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "Be29b4af3f7a5", -1.0, -1);
  graph.write_put_edge("", "C958e7588ae1c", "U38fdca6685ca", 1.0, -1);
  graph.write_put_edge("", "B23b74174e659", "U95f3426b8e5d", 1.0, -1);
  graph.write_put_edge("", "U47b466d57da1", "Bad1c69de7837", -3.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "Ca0a6aea6c82e", 1.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "B491d307dfe01", 1.0, -1);
  graph.write_put_edge("", "U79466f73dc0c", "B9c01ce5718d1", -6.0, -1);
  graph.write_put_edge("", "U704bd6ecde75", "C3b855f713d19", 1.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "C0b19d314485e", 4.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "Ce49159fe9d01", "U6661263fb410", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B8a531802473b", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B310b66ab31fb", 4.0, -1);
  graph.write_put_edge("", "U499f24158a40", "B491d307dfe01", 1.0, -1);
  graph.write_put_edge("", "Ud04c89aaf453", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "U9605bd4d1218", "B5a1c1d3d0140", 2.0, -1);
  graph.write_put_edge("", "Ud04c89aaf453", "B73a44e2bbd44", 4.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B9c01ce5718d1", 1.0, -1);
  graph.write_put_edge("", "Uc4ebbce44401", "Bed5126bc655d", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B79efabc4d8bf", 1.0, -1);
  graph.write_put_edge("", "Be5bb2f3d56cb", "U3c63a9b6115a", 1.0, -1);
  graph.write_put_edge("", "U8842ed397bb7", "C89c123f7bcf5", 1.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "B7f628ad203b5", 8.0, -1);
  graph.write_put_edge("", "U0f63ee3db59b", "Cc616eded7a99", 1.0, -1);
  graph.write_put_edge("", "U26aca0e369c7", "B45d72e29f004", 1.0, -1);
  graph.write_put_edge("", "Ubeded808a9c0", "B9c01ce5718d1", 6.0, -1);
  graph.write_put_edge("", "B30bf91bf5845", "Ue6cc7bfa0efd", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "Cb95e21215efa", 1.0, -1);
  graph.write_put_edge("", "C16dfdd8077c8", "U83282a51b600", 1.0, -1);
  graph.write_put_edge("", "C1f41b842849c", "U99a0f1f7e6ee", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B60d725feca77", -1.0, -1);
  graph.write_put_edge("", "B3f6f837bc345", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U704bd6ecde75", "Cfd47f43ac9cf", 1.0, -1);
  graph.write_put_edge("", "U4a82930ca419", "C472b59eeafa5", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ub93799d9400e", 1.0, -1);
  graph.write_put_edge("", "B47cc49866c37", "Uf5ee43a1b729", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B491d307dfe01", 3.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "C3fd1fdebe0e9", 1.0, -1);
  graph.write_put_edge("", "U0e6659929c53", "C55a114ca6e7c", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U638f5c19326f", "C63e21d051dda", 1.0, -1);
  graph.write_put_edge("", "U6240251593cd", "B75a44a52fa29", 4.0, -1);
  graph.write_put_edge("", "C7986cd8a648a", "U682c3380036f", 1.0, -1);
  graph.write_put_edge("", "C637133747308", "Ue202d5b01f8d", 1.0, -1);
  graph.write_put_edge("", "U9605bd4d1218", "B9c01ce5718d1", 2.0, -1);
  graph.write_put_edge("", "B68247950d9c0", "U9ce5721e93cf", 1.0, -1);
  graph.write_put_edge("", "Bf843e315d71b", "Uf6ce05bc4e5a", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "B5a1c1d3d0140", 5.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B5eb4c6be535a", -1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B79efabc4d8bf", 2.0, -1);
  graph.write_put_edge("", "B4115d364e05b", "Uf8bf10852d43", 1.0, -1);
  graph.write_put_edge("", "Ue40b938f47a4", "B8120aa1edccb", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "B499bfc56e77b", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "Cd172fb3fdc41", 1.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "C0b19d314485e", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "B5eb4c6be535a", -1.0, -1);
  graph.write_put_edge("", "Bad1c69de7837", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Bf34ee3bfc12b", 3.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "C2bbd63b00224", 1.0, -1);
  graph.write_put_edge("", "U35eb26fc07b4", "Cb117f464e558", -1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Cc42c3eeb9d20", 1.0, -1);
  graph.write_put_edge("", "C78ad459d3b81", "U9a2c85753a6d", 1.0, -1);
  graph.write_put_edge("", "Uf2b0a6b1d423", "C3fd1fdebe0e9", 7.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "B7f628ad203b5", 9.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Be2b46c17f1da", -1.0, -1);
  graph.write_put_edge("", "U83e829a2e822", "Bad1c69de7837", -4.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "C35678a54ef5f", 5.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "C9462ca240ceb", "Uf5096f6ab14e", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "C4893c40e481d", -1.0, -1);
  graph.write_put_edge("", "U18a178de1dfb", "Bf34ee3bfc12b", 1.0, -1);
  graph.write_put_edge("", "Bfae1726e4e87", "Uadeb43da4abb", 1.0, -1);
  graph.write_put_edge("", "U3de789cac826", "C3c17b70c3357", 1.0, -1);
  graph.write_put_edge("", "U6661263fb410", "Ce49159fe9d01", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B310b66ab31fb", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "C6aebafa4fe8e", 6.0, -1);
  graph.write_put_edge("", "U21769235b28d", "C6d52e861b366", 1.0, -1);
  graph.write_put_edge("", "Uad577360d968", "C6a2263dc469e", 5.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "Bad1c69de7837", 9.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "C6aebafa4fe8e", 1.0, -1);
  graph.write_put_edge("", "Ueb139752b907", "B1533941e2773", 1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "Be2b46c17f1da", -1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B92e4a185c654", 1.0, -1);
  graph.write_put_edge("", "Cbcf72c7e6061", "U0f63ee3db59b", 1.0, -1);
  graph.write_put_edge("", "Cf92f90725ffc", "U6661263fb410", 1.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "B79efabc4d8bf", 3.0, -1);
  graph.write_put_edge("", "U526f361717a8", "B9c01ce5718d1", 0.0, -1);
  graph.write_put_edge("", "Ud9df8116deba", "B310b66ab31fb", 1.0, -1);
  graph.write_put_edge("", "U6661263fb410", "C22e1102411ce", 1.0, -1);
  graph.write_put_edge("", "U21769235b28d", "C8ece5c618ac1", 1.0, -1);
  graph.write_put_edge("", "U99a0f1f7e6ee", "B10d3f548efc4", 1.0, -1);
  graph.write_put_edge("", "Ce06bda6030fe", "U362d375c067c", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Bf3a0a1165271", -1.0, -1);
  graph.write_put_edge("", "C5167c9b3d347", "U362d375c067c", 1.0, -1);
  graph.write_put_edge("", "C613f00c1333c", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B0a87a669fc28", 3.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "C588ffef22463", 4.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "C9028c7415403", -1.0, -1);
  graph.write_put_edge("", "Ue40b938f47a4", "B944097cdd968", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B310b66ab31fb", 1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "U362d375c067c", "C5060d0101429", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "Ce1a7d8996eb0", 2.0, -1);
  graph.write_put_edge("", "Uad577360d968", "C399b6349ab02", 6.0, -1);
  graph.write_put_edge("", "C992d8370db6b", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Uf6ce05bc4e5a", "B9c01ce5718d1", 1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "C15d8dfaceb75", 8.0, -1);
  graph.write_put_edge("", "Ub93799d9400e", "B491d307dfe01", 2.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Cac6ca02355da", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "C4f2dafca724f", 5.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "U8842ed397bb7", "C8c753f46c014", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B73a44e2bbd44", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "C8d80016b8292", 1.0, -1);
  graph.write_put_edge("", "C7a807e462b65", "Ud7002ae5a86c", 1.0, -1);
  graph.write_put_edge("", "C481cd737c873", "U21769235b28d", 1.0, -1);
  graph.write_put_edge("", "Uf3b5141d73f3", "B9c01ce5718d1", -3.0, -1);
  graph.write_put_edge("", "U72f88cf28226", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Bd49e3dac97b0", "Uadeb43da4abb", 1.0, -1);
  graph.write_put_edge("", "C0166be581dd4", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Cd172fb3fdc41", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U79466f73dc0c", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "U362d375c067c", "C5167c9b3d347", 1.0, -1);
  graph.write_put_edge("", "Ue6cc7bfa0efd", "Bed5126bc655d", 7.0, -1);
  graph.write_put_edge("", "U8842ed397bb7", "C789dceb76123", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "Ccbd85b8513f3", 1.0, -1);
  graph.write_put_edge("", "Cf77494dc63d7", "U38fdca6685ca", 1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "B0e230e9108dd", 1.0, -1);
  graph.write_put_edge("", "Cd4417a5d718e", "Ub93799d9400e", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Ud9df8116deba", 1.0, -1);
  graph.write_put_edge("", "Cb14487d862b3", "Uf5096f6ab14e", 1.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "C588ffef22463", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "C588ffef22463", 5.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "C78d6fac93d00", -1.0, -1);
  graph.write_put_edge("", "U59abf06369c3", "Be2b46c17f1da", -1.0, -1);
  graph.write_put_edge("", "Ucb84c094edba", "B491d307dfe01", 0.0, -1);
  graph.write_put_edge("", "Ubeded808a9c0", "B7f628ad203b5", -9.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "Ue7a29d5409f2", 1.0, -1);
  graph.write_put_edge("", "U4ba2e4e81c0e", "Caa62fc21e191", 1.0, -1);
  graph.write_put_edge("", "Ub93799d9400e", "Ccae34b3da05e", 1.0, -1);
  graph.write_put_edge("", "U0e6659929c53", "C6d52e861b366", -1.0, -1);
  graph.write_put_edge("", "U38fdca6685ca", "C0f834110f700", 1.0, -1);
  graph.write_put_edge("", "B92e4a185c654", "U41784ed376c3", 1.0, -1);
  graph.write_put_edge("", "B5a1c1d3d0140", "Uc3c31b8a022f", 1.0, -1);
  graph.write_put_edge("", "C6a2263dc469e", "Uf2b0a6b1d423", 1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "Cbce32a9b256a", 6.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "C3e84102071d1", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "C94bb73c10a06", 1.0, -1);
  graph.write_put_edge("", "C4f2dafca724f", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "U4f530cfe771e", "B7f628ad203b5", 0.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B75a44a52fa29", 1.0, -1);
  graph.write_put_edge("", "Ud5f1a29622d1", "B7f628ad203b5", 1.0, -1);
  graph.write_put_edge("", "Cbbf2df46955b", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "B5eb4c6be535a", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "C7062e90f7422", 1.0, -1);
  graph.write_put_edge("", "U99a0f1f7e6ee", "C279db553a831", 1.0, -1);
  graph.write_put_edge("", "C15d8dfaceb75", "U9e42f6dab85a", 1.0, -1);
  graph.write_put_edge("", "Ca0a6aea6c82e", "U016217c34c6e", 1.0, -1);
  graph.write_put_edge("", "Uc3c31b8a022f", "B5a1c1d3d0140", 1.0, -1);
  graph.write_put_edge("", "U35eb26fc07b4", "B7f628ad203b5", -2.0, -1);
  graph.write_put_edge("", "Ue40b938f47a4", "Cb3c476a45037", 1.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "C070e739180d6", 8.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B8a531802473b", -1.0, -1);
  graph.write_put_edge("", "U6661263fb410", "Ccb7dc40f1513", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Cb07d467c1c5e", 1.0, -1);
  graph.write_put_edge("", "C789dceb76123", "U8842ed397bb7", 1.0, -1);
  graph.write_put_edge("", "Uad577360d968", "U389f9f24b31c", 1.0, -1);
  graph.write_put_edge("", "C54972a5fbc16", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Be7bc0cfecab3", "U95f3426b8e5d", 1.0, -1);
  graph.write_put_edge("", "Bb78026d99388", "U9a89e0679dec", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "B25c85fe0df2d", 5.0, -1);
  graph.write_put_edge("", "U79466f73dc0c", "Bad1c69de7837", 2.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ba3c4a280657d", 3.0, -1);
  graph.write_put_edge("", "C6d52e861b366", "U21769235b28d", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "C6acd550a4ef3", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "B60d725feca77", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "B63fbe1427d09", 1.0, -1);
  graph.write_put_edge("", "U8aa2e2623fa5", "C7c4d9ca4623e", 1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "C6d52e861b366", -1.0, -1);
  graph.write_put_edge("", "C30e7409c2d5f", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B491d307dfe01", 1.0, -1);
  graph.write_put_edge("", "C801f204d0da8", "U21769235b28d", 1.0, -1);
  graph.write_put_edge("", "C5782d559baad", "U0cd6bd2dde4f", 1.0, -1);
  graph.write_put_edge("", "U41784ed376c3", "B92e4a185c654", 1.0, -1);
  graph.write_put_edge("", "U26aca0e369c7", "Cb117f464e558", 6.0, -1);
  graph.write_put_edge("", "U704bd6ecde75", "Cdd49e516723a", 1.0, -1);
  graph.write_put_edge("", "Ucbd309d6fcc0", "B5e7178dd70bb", 1.0, -1);
  graph.write_put_edge("", "Uf8bf10852d43", "B19d70698e3d8", 1.0, -1);
  graph.write_put_edge("", "C5060d0101429", "U362d375c067c", 1.0, -1);
  graph.write_put_edge("", "B253177f84f08", "Uf8bf10852d43", 1.0, -1);
  graph.write_put_edge("", "U34252014c05b", "Bb1e3630d2f4a", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "Cb14487d862b3", 6.0, -1);
  graph.write_put_edge("", "Cc01e00342d63", "U6661263fb410", 1.0, -1);
  graph.write_put_edge("", "C10872dc9b863", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Be29b4af3f7a5", -1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "C4818c4ed20bf", 1.0, -1);
  graph.write_put_edge("", "C3fd1fdebe0e9", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "U11456af7d414", "Bad1c69de7837", -2.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "C992d8370db6b", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "B45d72e29f004", 3.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "B3b3f2ecde430", -1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "Bc4addf09b79f", 3.0, -1);
}

fn put_testing_edges_2(graph: &mut AugMultiGraph) {
  graph.write_put_edge("", "U95f3426b8e5d", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U77a03e9a08af", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ub47d8c364c9e", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "U0be96c3b9883", "U5d33a9be1633", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "U9a89e0679dec", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "U0c17798eaab4", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "Udece0afd9a8b", -1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "Uadeb43da4abb", -1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Uc3c31b8a022f", -1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "Udece0afd9a8b", 1.0, -1);
  graph.write_put_edge("", "U5d33a9be1633", "U0be96c3b9883", 1.0, -1);
  graph.write_put_edge("", "U0be96c3b9883", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "U7725640de5b2", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U323855718209", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U7725640de5b2", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "U7725640de5b2", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U3ea0a229ad85", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "U3ea0a229ad85", "U0be96c3b9883", 1.0, -1);
  graph.write_put_edge("", "U3ea0a229ad85", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "U425e5e1ff39b", "U76a293d70033", 1.0, -1);
  graph.write_put_edge("", "Ucf8af4b3fa12", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U1691db0d4d7f", "U3ea0a229ad85", 1.0, -1);
  graph.write_put_edge("", "U3ea0a229ad85", "U1691db0d4d7f", 1.0, -1);
  graph.write_put_edge("", "U76a293d70033", "U425e5e1ff39b", 1.0, -1);
  graph.write_put_edge("", "Ub9713d01f478", "Ud10b3f42f87b", 1.0, -1);
  graph.write_put_edge("", "Ucc6cc40df2b7", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U0da9b1b0859f", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U0da9b1b0859f", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "U6307b2993c24", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ue925856b9cd9", "Ub4b17bc06434", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "Uc3c31b8a022f", -1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "U9e42f6dab85a", -1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Uaa4e2be7a87a", -1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "U1c285703fc63", -1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "U02fbd7c8df4c", 1.0, -1);
  graph.write_put_edge("", "U682c3380036f", "U6240251593cd", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Ud9df8116deba", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "U1e41b5f3adff", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ud04c89aaf453", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ud9df8116deba", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ub93799d9400e", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ud5b22ebf52f2", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U6240251593cd", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U016217c34c6e", 1.0, -1);
  graph.write_put_edge("", "Uc3c31b8a022f", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "Ue7a29d5409f2", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "U016217c34c6e", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U9a2c85753a6d", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U9e42f6dab85a", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "Uc3c31b8a022f", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "Uf5096f6ab14e", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "U9e42f6dab85a", 1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Udece0afd9a8b", 1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "U389f9f24b31c", 1.0, -1);
  graph.write_put_edge("", "U9e42f6dab85a", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "Uadeb43da4abb", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Uf2b0a6b1d423", 1.0, -1);
  graph.write_put_edge("", "Uad577360d968", "U389f9f24b31c", 1.0, -1);
  graph.write_put_edge("", "U77a03e9a08af", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "U71deb40da828", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "U71deb40da828", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "Uce7e9acd408e", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Uce7e9acd408e", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U04d86c56e1b8", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ud10b3f42f87b", "Ub9713d01f478", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U1691db0d4d7f", 1.0, -1);
  graph.write_put_edge("", "U75cddb09a54e", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U75cddb09a54e", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U0bf7eddae79d", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "Ue925856b9cd9", "Ucc76e1b73be0", 1.0, -1);
  graph.write_put_edge("", "U79466f73dc0c", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "U0cd6bd2dde4f", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "U09cf1f359454", 1.0, -1);
  graph.write_put_edge("", "Uf82dbb4708ba", "U0ae9f5d0bf02", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U3ea0a229ad85", 1.0, -1);
  graph.write_put_edge("", "U04d86c56e1b8", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ucb442106b78c", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U3a466eaf5798", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ue925856b9cd9", "U1691db0d4d7f", 1.0, -1);
  graph.write_put_edge("", "Ue925856b9cd9", "U3a466eaf5798", 1.0, -1);
  graph.write_put_edge("", "U0ae9f5d0bf02", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "Ueb139752b907", "U79466f73dc0c", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U0be96c3b9883", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U0cd6bd2dde4f", 0.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U79466f73dc0c", 1.0, -1);
  graph.write_put_edge("", "U76a293d70033", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U7b30e21179fc", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ua37f245cf686", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ua37f245cf686", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ua37f245cf686", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "U3be62375581d", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U47746fcce8c0", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Ud9df8116deba", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "B191f781ace43", 1.0, -1);
  graph.write_put_edge("", "Ucc76e1b73be0", "B83ef002b8120", 1.0, -1);
  graph.write_put_edge("", "U0ae9f5d0bf02", "Bed48703df71d", 1.0, -1);
  graph.write_put_edge("", "Uf82dbb4708ba", "B91796a98a225", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Bca63d8a2057b", 1.0, -1);
}

#[test]
fn encoding_serde() {
  let in_command: String = "foo".into();
  let in_context: &str = "bar";
  let in_arg1: &str = "baz";
  let in_arg2: &str = "bus";

  let payload = rmp_serde::to_vec(&(
    in_command.clone(),
    in_context,
    rmp_serde::to_vec(&(in_arg1, in_arg2)).unwrap(),
  ))
  .unwrap();

  let out_command: &str;
  let out_context: String;
  let _out_args: Vec<u8>;

  (out_command, out_context, _out_args) =
    rmp_serde::from_slice(payload.as_slice()).unwrap();

  assert_eq!(out_command, in_command);
  assert_eq!(out_context, in_context);
}

#[test]
fn encoding_response() {
  let foo = ("foo".to_string(), 1, 2, 3);
  let payload = encode_response(&foo).unwrap();

  let bar: (String, i32, i32, i32) = decode_response(&payload).unwrap();

  assert_eq!(foo.0, bar.0);
  assert_eq!(foo.1, bar.1);
  assert_eq!(foo.2, bar.2);
  assert_eq!(foo.3, bar.3);
}

#[test]
fn no_assert() {
  assert_eq!(meritrank_core::constants::ASSERT, false);
}

#[test]
fn recalculate_zero_graph_all() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "B7f628ad203b5", false, 0, 10000);

  let n = res.len();

  println!("Got {} edges", n);

  assert!(n > 1);
  assert!(n < 5);
}

#[test]
fn recalculate_out_of_bounds_regression() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 1.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);

  graph.write_recalculate_zero();
}

#[test]
fn graph_sort_order() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "Bfae1726e4e87", false, 0, 10000);

  assert!(res.len() > 1);

  for n in 1..res.len() {
    assert!(res[n - 1].2.abs() >= res[n].2.abs());
  }
}

#[test]
fn recalculate_zero_graph_duplicates() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> =
    graph.read_graph("", "Bb5f87c1621d5", "Ub01f4ad1b03f", false, 0, 10000);

  assert!(res.len() > 1);

  for (i, x) in res.iter().enumerate() {
    for (j, y) in res.iter().take(i).enumerate() {
      if x.0 == y.0 && x.1 == y.1 {
        println!("Duplicate: [{}, {}] {} -> {}", i, j, x.0, x.1);
      }
      assert!(x.0 != y.0 || x.1 != y.1);
    }
  }
}

#[test]
fn recalculate_zero_graph_positive_only() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "B7f628ad203b5", true, 0, 10000);

  let n = res.len();

  println!("Got {} edges", n);
  assert!(n > 1);
  assert!(n < 5);
}

#[test]
fn recalculate_zero_graph_focus_beacon() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> =
    graph.read_graph("", "U95f3426b8e5d", "B79efabc4d8bf", true, 0, 10000);

  let n = res.len();

  println!("Got {} edges", n);

  for edge in res {
    println!("{} -> {}", edge.0, edge.1);
  }

  assert!(n >= 2);
  assert!(n < 80);
}

#[test]
fn recalculate_zero_reset_perf() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph);
  graph.write_recalculate_zero();
  graph.reset();
  put_testing_edges(&mut graph);
  graph.write_create_context("X");
  graph.write_create_context("Y");
  graph.write_create_context("Z");
  graph.write_recalculate_zero();

  let begin = SystemTime::now();
  let get_time =
    || SystemTime::now().duration_since(begin).unwrap().as_millis();

  let res: Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "B0e230e9108dd", true, 0, 10000);

  assert!(res.len() > 1);

  assert!(get_time() < 300);
}

#[test]
fn recalculate_zero_scores() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> = graph.read_scores(
    "",
    "Uadeb43da4abb",
    "B",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  println!("Got {} edges", n);
  assert!(n > 5);
  assert!(n < 80);
}

#[test]
fn scores_sort_order() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph);

  graph.write_recalculate_zero();

  let res: Vec<_> = graph.read_scores(
    "",
    "Uadeb43da4abb",
    "B",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  assert!(res.len() > 1);

  for n in 1..res.len() {
    assert!(res[n - 1].2.abs() >= res[n].2.abs());
  }
}

#[test]
fn scores_without_recalculate() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges_2(&mut graph);

  graph.write_put_edge("", "U1", "U0", 1.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  assert_eq!(n, 2);
}

#[test]
fn scores_with_recalculate() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges_2(&mut graph);

  graph.write_recalculate_zero();

  graph.write_put_edge("", "U1", "U0", 1.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  assert!(n > 2);
}

#[test]
fn new_user_without_recalculate() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges_2(&mut graph);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  assert_eq!(n, 1); // Only self-score
}

#[test]
fn new_user_with_recalculate() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges_2(&mut graph);

  graph.write_recalculate_zero();

  //  read_scores should return zero opinion data even if the node doesn't exist

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  let n = res.len();

  assert!(n > 2);
}

#[test]
fn new_friend_smol() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "Ue925856b9cd9", "Ucc76e1b73be0", 1.0, -1);

  graph.write_recalculate_zero();

  let (_, _, s0, _, _, _) =
    graph.read_node_score("", "Ue925856b9cd9", "U6d2f25cc4264")[0];

  graph.write_put_edge("", "Ue925856b9cd9", "U6d2f25cc4264", 1.0, -1);

  let (_, _, s1, _, _, _) =
    graph.read_node_score("", "Ue925856b9cd9", "U6d2f25cc4264")[0];

  assert_ne!(s0, s1);
}

#[test]
fn new_friend_big() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges_2(&mut graph);

  graph.write_recalculate_zero();

  let (_, _, s0, _, _, _) =
    graph.read_node_score("", "Ue925856b9cd9", "U6d2f25cc4264")[0];

  graph.write_put_edge("", "Ue925856b9cd9", "U6d2f25cc4264", 1.0, -1);

  let (_, _, s1, _, _, _) =
    graph.read_node_score("", "Ue925856b9cd9", "U6d2f25cc4264")[0];

  assert_ne!(s0, s1);
}

#[test]
fn edge_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 1.5, -1);

  let edges: Vec<_> = graph.read_edges("");

  let edges_expected: Vec<(String, String, Weight)> =
    vec![("U1".to_string(), "U2".to_string(), 1.5)];

  assert_eq!(edges, edges_expected);
}

#[test]
fn edge_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 1.5, -1);

  let edges: Vec<_> = graph.read_edges("X");

  let edges_expected: Vec<(String, String, Weight)> =
    vec![("U1".to_string(), "U2".to_string(), 1.5)];

  assert_eq!(edges, edges_expected);
}

#[test]
fn null_context_is_sum() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "B1", "U2", 1.0, -1);
  graph.write_put_edge("Y", "B1", "U2", 2.0, -1);

  let edges: Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected: Vec<(String, String, Weight)> =
    vec![("B1".to_string(), "U2".to_string(), 3.0)];

  assert_eq!(edges, edges_expected);
}

#[test]
fn null_context_contains_all_users() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 1.0, -1);
  graph.write_put_edge("Y", "U1", "U3", 2.0, -1);

  let edges: Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected: Vec<(String, String, Weight)> = vec![
    ("U1".to_string(), "U2".to_string(), 1.0),
    ("U1".to_string(), "U3".to_string(), 2.0),
  ];

  assert_eq!(edges, edges_expected);
}

#[test]
fn user_edges_dup() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 1.0, -1);
  graph.write_put_edge("X", "U1", "U3", 2.0, -1);
  graph.write_create_context("Y");

  let edges: Vec<(String, String, Weight)> = graph.read_edges("Y");

  let edges_expected: Vec<(String, String, Weight)> = vec![
    ("U1".to_string(), "U2".to_string(), 1.0),
    ("U1".to_string(), "U3".to_string(), 2.0),
  ];

  assert_eq!(edges, edges_expected);
}

#[test]
fn non_user_edges_no_dup() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "C2", 1.0, -1);
  graph.write_put_edge("X", "U1", "C3", 2.0, -1);
  graph.write_create_context("Y");

  let edges: Vec<(String, String, Weight)> = graph.read_edges("Y");

  assert_eq!(edges.len(), 0);
}

#[test]
fn delete_nodes() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 1.0, -1);
  graph.write_delete_node("", "U1", -1);
  graph.write_delete_node("", "U2", -1);

  assert_eq!(graph.read_edges("").len(), 0);
}

#[test]
fn delete_contexted_edge() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "B1", "U2", 1.0, -1);
  graph.write_put_edge("Y", "B1", "U2", 2.0, -1);
  graph.write_delete_edge("X", "B1", "U2", -1);

  let edges: Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected: Vec<(String, String, Weight)> =
    vec![("B1".to_string(), "U2".to_string(), 2.0)];

  assert_eq!(edges, edges_expected);
}

#[test]
fn null_context_invariant() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "B1", "B2", 1.0, -1);
  graph.write_put_edge("Y", "B1", "B2", 2.0, -1);
  graph.write_delete_edge("X", "B1", "B2", -1);
  graph.write_put_edge("X", "B1", "B2", 1.0, -1);

  let edges: Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected: Vec<(String, String, Weight)> =
    vec![("B1".to_string(), "B2".to_string(), 3.0)];

  assert_eq!(edges, edges_expected);
}

#[test]
fn scores_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    false,
    10.0,
    false,
    0.0,
    false,
    0,
    u32::MAX,
  );

  assert_eq!(res.len(), 3);

  for x in res {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.1);
        assert!(x.2 < 0.4);
      },

      "U2" => {
        assert!(x.2 > 0.1);
        assert!(x.2 < 0.4);
      },

      "U3" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      _ => assert!(false),
    }
  }
}

#[test]
fn scores_reversed() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);
  graph.write_put_edge("", "U2", "U1", 4.0, -1);
  graph.write_put_edge("", "U3", "U1", -5.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    false,
    10.0,
    false,
    0.0,
    false,
    0,
    u32::MAX,
  );

  assert!(res.len() >= 2);
  assert!(res.len() <= 3);

  for x in res {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.0);
        assert!(x.2 < 0.4);
        assert!(x.3 > 0.0);
        assert!(x.3 < 0.4);
      },

      "U2" => {
        assert!(x.2 > -0.1);
        assert!(x.2 < 0.3);
        assert!(x.3 > -0.3);
        assert!(x.3 < 0.1);
      },

      "U3" => {
        assert!(x.2 > -0.1);
        assert!(x.2 < 0.3);
        assert!(x.3 > -0.6);
        assert!(x.3 < 0.0);
      },

      _ => assert!(false),
    }
  }
}

#[test]
fn scores_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("X", "U2", "U3", 3.0, -1);

  let res: Vec<_> = graph.read_scores(
    "X",
    "U1",
    "U",
    false,
    10.0,
    false,
    0.0,
    false,
    0,
    u32::MAX,
  );

  assert_eq!(res.len(), 3);

  for x in res {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      "U2" => {
        assert!(x.2 > 0.1);
        assert!(x.2 < 0.4);
      },

      "U3" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.5);
      },

      _ => assert!(false),
    }
  }
}

#[test]
fn scores_unknown_context() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "B1", "B2", 2.0, -1);
  graph.write_put_edge("X", "B1", "B3", 1.0, -1);
  graph.write_put_edge("X", "B2", "B3", 3.0, -1);

  let res: Vec<_> = graph.read_scores(
    "Y",
    "B1",
    "B",
    false,
    10.0,
    false,
    0.0,
    false,
    0,
    u32::MAX,
  );

  assert_eq!(res.len(), 0);
}

#[test]
fn scores_reset_smoke() {
  let mut graph_read = AugMultiGraph::new();
  let mut graph_write = AugMultiGraph::new();

  graph_write.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph_write.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph_write.write_put_edge("X", "U2", "U3", 3.0, -1);

  graph_read.copy_from(&graph_write);
  let res: Vec<_> = graph_read.read_scores(
    "X", "U1", "U", false, 10.0, false, 0.0, false, 0, 2147483647,
  );

  assert_eq!(res.len(), 3);

  graph_write.reset();

  graph_write.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph_write.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph_write.write_put_edge("X", "U2", "U3", 3.0, -1);

  graph_read.copy_from(&graph_write);
  let res: Vec<_> = graph_read.read_scores(
    "X",
    "U1",
    "U",
    false,
    2147483647.0,
    false,
    -2147483648.0,
    false,
    0,
    2147483647,
  );

  assert_eq!(res.len(), 3);
}

#[test]
fn scores_self() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "B1", "B2", 2.0, -1);
  graph.write_put_edge("X", "B1", "B3", 1.0, -1);
  graph.write_put_edge("X", "B2", "U1", 3.0, -1);
  graph.write_create_context("Y");

  let res: Vec<_> = graph.read_scores(
    "Y",
    "U1",
    "U",
    false,
    10.0,
    false,
    0.0,
    false,
    0,
    u32::MAX,
  );

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U1");
  assert!(res[0].2 > 0.999);
  assert!(res[0].2 < 1.001);
}

#[test]
fn node_list_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U3", "U2", 3.0, -1);

  let res: Vec<(String,)> = graph.read_node_list();

  let mut has_u1 = false;
  let mut has_u2 = false;
  let mut has_u3 = false;

  for (x,) in res {
    match x.as_str() {
      "U1" => has_u1 = true,
      "U2" => has_u2 = true,
      "U3" => has_u3 = true,
      _ => assert!(false),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
  assert!(has_u3);
}

#[test]
fn node_list_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("X", "U3", "U2", 3.0, -1);

  let res: Vec<(String,)> = graph.read_node_list();

  let mut has_u1 = false;
  let mut has_u2 = false;
  let mut has_u3 = false;

  for (x,) in res {
    match x.as_str() {
      "U1" => has_u1 = true,
      "U2" => has_u2 = true,
      "U3" => has_u3 = true,
      _ => assert!(false),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
  assert!(has_u3);
}

#[test]
fn node_list_mixed() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("Y", "U3", "U2", 3.0, -1);

  let res: Vec<(String,)> = graph.read_node_list();

  let mut has_u1 = false;
  let mut has_u2 = false;
  let mut has_u3 = false;

  for (x,) in res {
    match x.as_str() {
      "U1" => has_u1 = true,
      "U2" => has_u2 = true,
      "U3" => has_u3 = true,
      _ => assert!(false),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
  assert!(has_u3);
}

#[test]
fn node_score_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U3", "U2", 3.0, -1);

  let res: Vec<_> = graph.read_node_score("", "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U2");
  assert!(res[0].2 > 0.25);
  assert!(res[0].2 < 0.4);
}

#[test]
fn node_score_reversed() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U3", "U2", 3.0, -1);
  graph.write_put_edge("", "U2", "U1", 4.0, -1);

  let res: Vec<_> = graph.read_node_score("", "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U2");
  assert!(res[0].2 > 0.2);
  assert!(res[0].2 < 0.4);
  assert!(res[0].3 > 0.2);
  assert!(res[0].3 < 0.4);
}

#[test]
fn node_score_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("X", "U3", "U2", 3.0, -1);

  let res: Vec<_> = graph.read_node_score("X", "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U2");
  assert!(res[0].2 > 0.3);
  assert!(res[0].2 < 0.45);
}

#[test]
fn mutual_scores_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 3.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U1", 2.0, -1);
  graph.write_put_edge("", "U2", "U3", 4.0, -1);
  graph.write_put_edge("", "U3", "U1", 3.0, -1);
  graph.write_put_edge("", "U3", "U2", 2.0, -1);

  let res: Vec<_> = graph.read_mutual_scores("", "U1");

  assert_eq!(res.len(), 3);

  let mut u1 = true;
  let mut u2 = true;
  let mut u3 = true;

  for x in res.iter() {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.4);
        assert!(x.3 > 0.25);
        assert!(x.3 < 0.4);
        assert!(u1);
        u1 = false;
      },

      "U2" => {
        assert!(x.2 > 0.15);
        assert!(x.2 < 0.3);
        assert!(x.3 > 0.15);
        assert!(x.3 < 0.3);
        assert!(u2);
        u2 = false;
      },

      "U3" => {
        assert!(x.2 > 0.15);
        assert!(x.2 < 0.3);
        assert!(x.3 > 0.15);
        assert!(x.3 < 0.3);
        assert!(u3);
        u3 = false;
      },

      _ => {
        assert!(false);
      },
    };
  }
}

#[test]
fn mutual_scores_self() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 3.0, -1);
  graph.write_delete_edge("", "U1", "U2", -1);

  let res: Vec<_> = graph.read_mutual_scores("", "U1");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U1");
  assert!(res[0].2 > 0.79);
  assert!(res[0].2 < 0.81);
  assert!(res[0].3 > 0.79);
  assert!(res[0].3 < 0.81);
}

#[test]
fn mutual_scores_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 3.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("X", "U2", "U1", 2.0, -1);
  graph.write_put_edge("X", "U2", "U3", 4.0, -1);
  graph.write_put_edge("X", "U3", "U1", 3.0, -1);
  graph.write_put_edge("X", "U3", "U2", 2.0, -1);

  let res: Vec<_> = graph.read_mutual_scores("X", "U1");

  assert_eq!(res.len(), 3);

  let mut u1 = true;
  let mut u2 = true;
  let mut u3 = true;

  for x in res.iter() {
    assert_eq!(x.0, "U1");

    match x.1.as_str() {
      "U1" => {
        assert!(x.2 > 0.3);
        assert!(x.2 < 0.5);
        assert!(x.3 > 0.3);
        assert!(x.3 < 0.5);
        assert!(u1);
        u1 = false;
      },

      "U2" => {
        assert!(x.2 > 0.25);
        assert!(x.2 < 0.4);
        assert!(x.3 > 0.2);
        assert!(x.3 < 0.35);
        assert!(u2);
        u2 = false;
      },

      "U3" => {
        assert!(x.2 > 0.2);
        assert!(x.2 < 0.35);
        assert!(x.3 > 0.2);
        assert!(x.3 < 0.35);
        assert!(u3);
        u3 = false;
      },

      _ => {
        assert!(false);
      },
    };
  }
}

#[test]
fn graph_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);

  let res: Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 2);

  let mut has_u1 = false;
  let mut has_u2 = false;

  for x in res {
    match x.0.as_str() {
      "U1" => {
        assert_eq!(x.1, "U2");
        assert!(x.2 > 0.65);
        assert!(x.2 < 0.67);
        has_u1 = true;
      },

      "U2" => {
        assert_eq!(x.1, "U3");
        assert!(x.2 > 0.99);
        assert!(x.2 < 1.01);
        has_u2 = true;
      },

      _ => panic!(),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
}

#[test]
fn graph_reversed() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);
  graph.write_put_edge("", "U2", "U1", 4.0, -1);

  let res: Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 3);

  for x in res {
    match x.0.as_str() {
      "U1" => {
        assert_eq!(x.1, "U2");
        assert!(x.2 > 0.6);
        assert!(x.2 < 0.7);
        assert!(x.3 > 0.05);
        assert!(x.3 < 0.3);
      },

      "U2" => {
        if x.1 == "U1" {
          assert!(x.2 > 0.5);
          assert!(x.2 < 0.6);
          assert!(x.3 > 0.2);
          assert!(x.3 < 0.5);
        }

        if x.1 == "U3" {
          assert!(x.2 > 0.39);
          assert!(x.2 < 0.49);
          assert!(x.3 > 0.1);
          assert!(x.3 < 0.3);
        }
      },

      _ => panic!(),
    }
  }
}

#[test]
fn graph_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 2.0, -1);
  graph.write_put_edge("X", "U1", "U3", 1.0, -1);
  graph.write_put_edge("X", "U2", "U3", 3.0, -1);

  let res: Vec<_> = graph.read_graph("X", "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 2);

  let mut has_u1 = false;
  let mut has_u2 = false;

  for x in res {
    match x.0.as_str() {
      "U1" => {
        assert_eq!(x.1, "U2");
        assert!(x.2 > 0.65);
        assert!(x.2 < 0.67);
        has_u1 = true;
      },

      "U2" => {
        assert_eq!(x.1, "U3");
        assert!(x.2 > 0.99);
        assert!(x.2 < 1.01);
        has_u2 = true;
      },

      _ => panic!(),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
}

#[test]
fn graph_empty() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);

  graph.write_delete_edge("", "U1", "U2", -1);
  graph.write_delete_edge("", "U1", "U3", -1);
  graph.write_delete_edge("", "U2", "U3", -1);

  let res: Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

  for x in res.iter() {
    println!("{} -> {}: {}", x.0, x.1, x.2);
  }

  assert_eq!(res.len(), 0);
}

#[test]
fn graph_removed_edge() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "B2", 1.0, -1);
  graph.write_put_edge("", "B2", "U1", 2.0, -1);
  graph.write_put_edge("", "B2", "C2", 1.0, -1);
  graph.write_put_edge("", "B2", "C3", 1.5, -1);
  graph.write_put_edge("", "B2", "C4", 3.0, -1);

  graph.write_delete_edge("", "U1", "B2", -1);

  let res: Vec<_> = graph.read_graph("", "U1", "B2", false, 0, 10000);

  for x in res.iter() {
    println!("{} -> {}: {}", x.0, x.1, x.2);
  }

  assert_eq!(res.len(), 0);
}

#[test]
fn new_edges_fetch() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 1.0, -1);

  assert_eq!(graph.write_fetch_new_edges("U1", "B").len(), 0);

  graph.write_put_edge("", "U1", "B3", 2.0, -1);
  graph.write_put_edge("", "U2", "B4", 3.0, -1);

  let beacons = graph.write_fetch_new_edges("U1", "B");

  assert_eq!(beacons.len(), 2);
  assert_eq!(beacons[0].0, "B3");
  assert_eq!(beacons[1].0, "B4");

  assert_eq!(graph.write_fetch_new_edges("U1", "B").len(), 0);
}

#[test]
fn new_edges_filter() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 1.0, -1);

  assert_eq!(graph.write_fetch_new_edges("U1", "B").len(), 0);

  graph.write_put_edge("", "U1", "B3", 2.0, -1);
  graph.write_put_edge("", "U2", "B4", 3.0, -1);

  let filter = graph.read_new_edges_filter("U1");
  assert_eq!(filter.len(), 32);

  let beacons = graph.write_fetch_new_edges("U1", "B");

  assert_eq!(beacons.len(), 2);
  assert_eq!(beacons[0].0, "B3");
  assert_eq!(beacons[1].0, "B4");

  graph.write_new_edges_filter("U1", &filter);
  let beacons = graph.write_fetch_new_edges("U1", "B");

  assert_eq!(beacons.len(), 2);
  assert_eq!(beacons[0].0, "B3");
  assert_eq!(beacons[1].0, "B4");
}

#[test]
fn copy_user_edges_into_context() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 1.0, -1);
  graph.write_put_edge("X", "U1", "C2", 2.0, -1);
  graph.write_create_context("Y");

  let edges: Vec<(String, String, Weight)> = graph.read_edges("Y");

  assert_eq!(edges.len(), 1);
  assert_eq!(edges[0].0, "U1");
  assert_eq!(edges[0].1, "U2");
  assert!(edges[0].2 > 0.999);
  assert!(edges[0].2 < 1.001);
}

#[test]
fn context_already_exist() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "C2", 1.0, -1);
  graph.write_create_context("X");

  let edges: Vec<(String, String, Weight)> = graph.read_edges("X");

  assert_eq!(edges.len(), 1);
  assert_eq!(edges[0].0, "U1");
  assert_eq!(edges[0].1, "C2");
  assert!(edges[0].2 > 0.999);
  assert!(edges[0].2 < 1.001);
}

#[test]
fn mutual_scores_cluster_single_score_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 10.0, -1);

  let res: Vec<_> = graph.read_mutual_scores("", "U1");

  println!("{:?}", res);

  assert_eq!(res.len(), 2);
  assert!(res[0].4 == 100);
  assert!(res[1].4 == 1);
}

#[test]
fn mutual_scores_cluster_single_score_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 10.0, -1);

  let res: Vec<_> = graph.read_mutual_scores("X", "U1");

  println!("{:?}", res);

  assert_eq!(res.len(), 2);
  assert!(res[0].4 == 100);
  assert!(res[1].4 == 1);
}

#[test]
fn mutual_scores_clustering() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", -5.0, -1);
  graph.write_put_edge("X", "U1", "U3", -5.0, -1);
  graph.write_put_edge("X", "U1", "U4", 1.0, -1);
  graph.write_put_edge("X", "U1", "U5", 1.0, -1);
  graph.write_put_edge("X", "U1", "U6", 3.0, -1);
  graph.write_put_edge("X", "U1", "U7", 3.0, -1);
  graph.write_put_edge("X", "U1", "U8", 5.0, -1);
  graph.write_put_edge("X", "U1", "U9", 5.0, -1);
  graph.write_put_edge("X", "U1", "U10", 6.0, -1);
  graph.write_put_edge("X", "U1", "U11", 6.0, -1);

  graph.write_put_edge("X", "U2", "U1", 1.0, -1);
  graph.write_put_edge("X", "U3", "U1", 2.0, -1);
  graph.write_put_edge("X", "U4", "U1", 3.0, -1);
  graph.write_put_edge("X", "U5", "U1", 1.0, -1);
  graph.write_put_edge("X", "U6", "U1", 2.0, -1);
  graph.write_put_edge("X", "U7", "U1", 3.0, -1);
  graph.write_put_edge("X", "U8", "U1", 1.0, -1);
  graph.write_put_edge("X", "U9", "U1", 2.0, -1);
  graph.write_put_edge("X", "U10", "U1", 3.0, -1);
  graph.write_put_edge("X", "U11", "U1", 1.0, -1);

  graph.write_put_edge("X", "U2", "U3", 4.0, -1);
  graph.write_put_edge("X", "U3", "U4", 5.0, -1);
  graph.write_put_edge("X", "U4", "U5", 6.0, -1);
  graph.write_put_edge("X", "U5", "U6", 1.0, -1);
  graph.write_put_edge("X", "U6", "U7", 2.0, -1);
  graph.write_put_edge("X", "U7", "U8", 3.0, -1);
  graph.write_put_edge("X", "U8", "U9", 4.0, -1);
  graph.write_put_edge("X", "U9", "U10", 5.0, -1);
  graph.write_put_edge("X", "U10", "U11", 6.0, -1);

  let res: Vec<_> = graph.read_mutual_scores("X", "U1");

  for (
    _src,
    _dst,
    _score_of_dst,
    _score_of_src,
    cluster_of_dst,
    cluster_of_src,
  ) in res.iter()
  {
    assert!(*cluster_of_dst >= 1);
    assert!(*cluster_of_dst <= 100);
    assert!(*cluster_of_src >= 1);
    assert!(*cluster_of_src <= 100);
  }
}

#[test]
fn five_user_scores_clustering() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 5.0, -1);
  graph.write_put_edge("", "U1", "U3", 1.0, -1);
  graph.write_put_edge("", "U1", "U4", 2.0, -1);
  graph.write_put_edge("", "U1", "U5", 3.0, -1);
  graph.write_put_edge("", "U2", "U1", 4.0, -1);

  //  We will get 5 score values including self-score.

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  println!("{:?}", res);

  assert_eq!(res.len(), 5);

  assert!(res[0].4 <= 100);
  assert!(res[0].4 >= 40);

  assert!(res[1].4 <= 100);
  assert!(res[1].4 >= 20);

  assert!(res[2].4 <= 100);
  assert!(res[2].4 >= 1);

  assert!(res[3].4 <= 80);
  assert!(res[3].4 >= 1);

  assert!(res[4].4 <= 60);
  assert!(res[4].4 >= 1);
}

#[test]
fn five_beacon_scores_clustering() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "B2", 5.0, -1);
  graph.write_put_edge("", "U1", "B3", 1.0, -1);
  graph.write_put_edge("", "U1", "B4", 2.0, -1);
  graph.write_put_edge("", "U1", "B5", 3.0, -1);
  graph.write_put_edge("", "U1", "B6", 3.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "B",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  println!("{:?}", res);

  assert_eq!(res.len(), 5);

  assert!(res[0].4 <= 100);
  assert!(res[0].4 >= 40);

  assert!(res[1].4 <= 100);
  assert!(res[1].4 >= 20);

  assert!(res[2].4 <= 100);
  assert!(res[2].4 >= 1);

  assert!(res[3].4 <= 80);
  assert!(res[3].4 >= 1);

  assert!(res[4].4 <= 60);
  assert!(res[4].4 >= 1);
}

#[test]
fn three_scores_chain_clustering() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U2", "U3", 3.0, -1);
  graph.write_put_edge("", "U3", "U1", 4.0, -1);

  //  We will get 3 score values including self-score.

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  println!("{:?}", res);

  assert_eq!(res.len(), 3);

  assert!(res[0].4 <= 100);
  assert!(res[0].4 >= 40);

  assert!(res[1].4 <= 80);
  assert!(res[1].4 >= 20);

  assert!(res[2].4 <= 60);
  assert!(res[2].4 >= 1);
}

#[test]
fn separate_clusters_without_users() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "B1", 3.0, -1);
  graph.write_put_edge("", "U1", "C1", 4.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  println!("{:?}", res);

  assert_eq!(res.len(), 3);

  assert_eq!(res[0].4, 100);
  assert_eq!(res[1].4, 100);
  assert_eq!(res[2].4, 100);
}

#[test]
fn separate_clusters_self_score() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0, -1);
  graph.write_put_edge("", "U1", "B1", 3.0, -1);
  graph.write_put_edge("", "U1", "C1", 4.0, -1);

  let res: Vec<_> = graph.read_scores(
    "",
    "U1",
    "U",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  println!("{:?}", res);

  assert_eq!(res.len(), 2);

  assert_eq!(res[0].4, 100);
  assert_eq!(res[1].4, 1);
}

#[test]
fn regression_delete_self_reference_panic() {
  let mut graph = AugMultiGraph::new();
  graph.write_put_edge("", "Ud57e58e4b20d", "U000000000000", 1.0, -1);
  graph.write_delete_edge("", "U000000000000", "U000000000000", -1);
}

#[test]
fn regression_beacons_clustering() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U95f3426b8e5d", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U77a03e9a08af", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ub47d8c364c9e", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "U0be96c3b9883", "U5d33a9be1633", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "U9a89e0679dec", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "U0c17798eaab4", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "Udece0afd9a8b", -1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "Uadeb43da4abb", -1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Uc3c31b8a022f", -1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "Udece0afd9a8b", 1.0, -1);
  graph.write_put_edge("", "U5d33a9be1633", "U0be96c3b9883", 1.0, -1);
  graph.write_put_edge("", "U0be96c3b9883", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "U7725640de5b2", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U323855718209", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U7725640de5b2", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "U7725640de5b2", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U3ea0a229ad85", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "U3ea0a229ad85", "U0be96c3b9883", 1.0, -1);
  graph.write_put_edge("", "U3ea0a229ad85", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "U425e5e1ff39b", "U76a293d70033", 1.0, -1);
  graph.write_put_edge("", "Ucf8af4b3fa12", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U1691db0d4d7f", "U3ea0a229ad85", 1.0, -1);
  graph.write_put_edge("", "U3ea0a229ad85", "U1691db0d4d7f", 1.0, -1);
  graph.write_put_edge("", "U76a293d70033", "U425e5e1ff39b", 1.0, -1);
  graph.write_put_edge("", "Ub9713d01f478", "Ud10b3f42f87b", 1.0, -1);
  graph.write_put_edge("", "Ucc6cc40df2b7", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U0da9b1b0859f", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U0da9b1b0859f", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "U6307b2993c24", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ue925856b9cd9", "Ub4b17bc06434", 1.0, -1);
  graph.write_put_edge("", "Ud57e58e4b20d", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ud57e58e4b20d", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "Ud57e58e4b20d", "U09cf1f359454", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "Udece0afd9a8b", "Uc3c31b8a022f", -1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "U9e42f6dab85a", -1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Uaa4e2be7a87a", -1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "U1c285703fc63", -1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U01814d1ec9ff", "U02fbd7c8df4c", 1.0, -1);
  graph.write_put_edge("", "U682c3380036f", "U6240251593cd", 1.0, -1);
  graph.write_put_edge("", "U6d2f25cc4264", "Ud9df8116deba", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "U1e41b5f3adff", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ud04c89aaf453", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ud9df8116deba", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ub93799d9400e", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "Ud5b22ebf52f2", 1.0, -1);
  graph.write_put_edge("", "U8a78048d60f7", "U6240251593cd", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U016217c34c6e", 1.0, -1);
  graph.write_put_edge("", "Uc3c31b8a022f", "U1c285703fc63", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "Ue7a29d5409f2", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "U016217c34c6e", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U9a2c85753a6d", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "U9e42f6dab85a", 1.0, -1);
  graph.write_put_edge("", "U389f9f24b31c", "Uc3c31b8a022f", 1.0, -1);
  graph.write_put_edge("", "U9a2c85753a6d", "Uf5096f6ab14e", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "U9e42f6dab85a", 1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Udece0afd9a8b", 1.0, -1);
  graph.write_put_edge("", "Uf5096f6ab14e", "U7a8d8324441d", 1.0, -1);
  graph.write_put_edge("", "U016217c34c6e", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "U389f9f24b31c", 1.0, -1);
  graph.write_put_edge("", "U9e42f6dab85a", "U80e22da6d8c4", 1.0, -1);
  graph.write_put_edge("", "Uaa4e2be7a87a", "Uadeb43da4abb", 1.0, -1);
  graph.write_put_edge("", "U0c17798eaab4", "Uad577360d968", 1.0, -1);
  graph.write_put_edge("", "Ue7a29d5409f2", "Uf2b0a6b1d423", 1.0, -1);
  graph.write_put_edge("", "Uad577360d968", "U389f9f24b31c", 1.0, -1);
  graph.write_put_edge("", "U77a03e9a08af", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "U71deb40da828", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "U71deb40da828", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "Uce7e9acd408e", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Uce7e9acd408e", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U04d86c56e1b8", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ud10b3f42f87b", "Ub9713d01f478", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U1691db0d4d7f", 1.0, -1);
  graph.write_put_edge("", "U75cddb09a54e", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U75cddb09a54e", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U0bf7eddae79d", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "Ue925856b9cd9", "Ucc76e1b73be0", 1.0, -1);
  graph.write_put_edge("", "U64962846d3e0", "Uc4ebbce44401", 1.0, -1);
  graph.write_put_edge("", "U79466f73dc0c", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "U0cd6bd2dde4f", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "U09cf1f359454", 1.0, -1);
  graph.write_put_edge("", "Uf82dbb4708ba", "U0ae9f5d0bf02", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U3ea0a229ad85", 1.0, -1);
  graph.write_put_edge("", "U04d86c56e1b8", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ucb442106b78c", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U3a466eaf5798", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ue925856b9cd9", "U1691db0d4d7f", 1.0, -1);
  graph.write_put_edge("", "Ue925856b9cd9", "U3a466eaf5798", 1.0, -1);
  graph.write_put_edge("", "Ue925856b9cd9", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U0ae9f5d0bf02", "Ub01f4ad1b03f", 1.0, -1);
  graph.write_put_edge("", "Ueb139752b907", "U79466f73dc0c", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U0be96c3b9883", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U0cd6bd2dde4f", 0.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U79466f73dc0c", 1.0, -1);
  graph.write_put_edge("", "U76a293d70033", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "U7b30e21179fc", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ua37f245cf686", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ua37f245cf686", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ua37f245cf686", "U55272fd6c264", 1.0, -1);
  graph.write_put_edge("", "U3be62375581d", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "U47746fcce8c0", "U499f24158a40", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U6d2f25cc4264", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Ud9df8116deba", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U01814d1ec9ff", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "U8a78048d60f7", 1.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "B191f781ace43", 1.0, -1);
  graph.write_put_edge("", "Ucc76e1b73be0", "B83ef002b8120", 1.0, -1);
  graph.write_put_edge("", "U0ae9f5d0bf02", "Bed48703df71d", 1.0, -1);
  graph.write_put_edge("", "Uf82dbb4708ba", "B91796a98a225", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Bca63d8a2057b", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B99e6c816679d", 1.0, -1);
  graph.write_put_edge("", "U0be96c3b9883", "Bea6112348aa2", 1.0, -1);
  graph.write_put_edge("", "U043fbd8f99fa", "Be3a19d76f8d5", 1.0, -1);
  graph.write_put_edge("", "U13f1bde6e566", "B751685059827", 1.0, -1);
  graph.write_put_edge("", "Udf36e098c231", "B037216730d08", 1.0, -1);
  graph.write_put_edge("", "Ua06b740988b1", "B2559d1345f88", 1.0, -1);
  graph.write_put_edge("", "U46e5959770ad", "B31c1f968221c", 1.0, -1);
  graph.write_put_edge("", "U3be62375581d", "B1e4f6f3a90c9", 1.0, -1);
  graph.write_put_edge("", "U784662a9d229", "Bf65b8bfd4efb", 1.0, -1);
  graph.write_put_edge("", "U422adbe0083e", "B2255a6be235d", 1.0, -1);
  graph.write_put_edge("", "Ub4b17bc06434", "B0d9995f89328", 1.0, -1);
  graph.write_put_edge("", "U9cf0aee30fd2", "Be616325b7d17", 1.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "Bc173d5552e2e", 1.0, -1);
  graph.write_put_edge("", "U95f3426b8e5d", "B24f9f2026cec", 1.0, -1);
  graph.write_put_edge("", "Uc4ebbce44401", "Bed5126bc655d", 1.0, -1);
  graph.write_put_edge("", "Ue6cc7bfa0efd", "B30bf91bf5845", 1.0, -1);
  graph.write_put_edge("", "U1bcba4fd7175", "Bc896788cd2ef", 1.0, -1);
  graph.write_put_edge("", "Uc3c31b8a022f", "B5a1c1d3d0140", 1.0, -1);
  graph.write_put_edge("", "U99a0f1f7e6ee", "B10d3f548efc4", 1.0, -1);
  graph.write_put_edge("", "Ue40b938f47a4", "B8120aa1edccb", 1.0, -1);
  graph.write_put_edge("", "U34252014c05b", "B19ea554faf29", 1.0, -1);
  graph.write_put_edge("", "U34252014c05b", "Bb1e3630d2f4a", 1.0, -1);
  graph.write_put_edge("", "Ue40b938f47a4", "B944097cdd968", 1.0, -1);
  graph.write_put_edge("", "U41784ed376c3", "B92e4a185c654", 1.0, -1);
  graph.write_put_edge("", "U1c285703fc63", "B63fbe1427d09", 1.0, -1);
  graph.write_put_edge("", "U1e41b5f3adff", "Ba5d64165e5d5", 1.0, -1);
  graph.write_put_edge("", "Ud04c89aaf453", "B4f14b223b56d", 1.0, -1);
  graph.write_put_edge("", "U3c63a9b6115a", "Be5bb2f3d56cb", 1.0, -1);
  graph.write_put_edge("", "U6240251593cd", "Bf34ee3bfc12b", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B70df5dbab8c3", 1.0, -1);
  graph.write_put_edge("", "U9e42f6dab85a", "B3c467fb437b2", 1.0, -1);
  graph.write_put_edge("", "U99a0f1f7e6ee", "Bd90a1cf73384", 1.0, -1);
  graph.write_put_edge("", "U34252014c05b", "B0a87a669fc28", 1.0, -1);
  graph.write_put_edge("", "Uef7fbf45ef11", "B25c85fe0df2d", 1.0, -1);
  graph.write_put_edge("", "U02fbd7c8df4c", "Bd7a8bfcf3337", 1.0, -1);
  graph.write_put_edge("", "Uc1158424318a", "Bdf39d0e1daf5", 1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "Bb78026d99388", 1.0, -1);
  graph.write_put_edge("", "Uc35c445325f5", "Be29b4af3f7a5", 1.0, -1);
  graph.write_put_edge("", "U0cd6bd2dde4f", "Bc4addf09b79f", 1.0, -1);
  graph.write_put_edge("", "U09cf1f359454", "B4f00e7813add", 1.0, -1);
  graph.write_put_edge("", "U9a89e0679dec", "Bf3a0a1165271", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "B60d725feca77", 1.0, -1);
  graph.write_put_edge("", "Uad577360d968", "B5eb4c6be535a", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "B79efabc4d8bf", 1.0, -1);
  graph.write_put_edge("", "U499f24158a40", "Ba3c4a280657d", 1.0, -1);
  graph.write_put_edge("", "U7a8d8324441d", "B7f628ad203b5", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "B7fbe3633b4b0", 1.0, -1);
  graph.write_put_edge("", "U79466f73dc0c", "B1533941e2773", 1.0, -1);
  graph.write_put_edge("", "Ub01f4ad1b03f", "Bdbd2f5c5e2bc", 1.0, -1);
  graph.write_put_edge("", "U55272fd6c264", "B8fabb952bc4b", 1.0, -1);
  graph.write_put_edge("", "U80e22da6d8c4", "Be2b46c17f1da", 1.0, -1);

  graph.write_recalculate_zero();

  let res: Vec<_> = graph.read_scores(
    "",
    "U6d2f25cc4264",
    "B",
    true,
    100.0,
    false,
    -100.0,
    false,
    0,
    u32::MAX,
  );

  for x in res.iter() {
    println!("{:?}", x);
  }

  let count = res.len();

  for (n, x) in res.iter().enumerate() {
    assert!(x.4 >= 1);
    assert!(x.4 <= 100);

    let percentile = (1 + ((count - n - 1) * 100) / count) as i32;

    assert!(x.4 >= percentile - 25);
    assert!(x.4 <= percentile + 25);
  }
}

#[test]
fn vsids_write_edge() {
  let mut graph = AugMultiGraph::new();
  graph.write_put_edge("", "U1", "U2", 3.0, 0);
  graph.write_put_edge("", "U1", "U3", 1.0, 20);
  let u12 = graph.read_node_score("", "U1", "U2");
  let u13 = graph.read_node_score("", "U1", "U3");
  assert!(u12[0].2 < u13[0].2, "Assert that thanks to magnitude, U3 has a higher score than U2");

  // Test deletion of too small edges
  graph.write_put_edge("", "U1", "U4", 1.0, 200);
  let u12_final = graph.read_node_score("", "U1", "U2");
  let u13_final = graph.read_node_score("", "U1", "U3");
  assert!(u12_final.is_empty() || u12_final[0].2 == 0.0, "U1->U2 edge should not exist");
  assert!(u13_final.is_empty() || u13_final[0].2 == 0.0, "U1->U3 edge should not exist");
}

#[test]
fn vsids_edges_churn() {
  let mut graph = AugMultiGraph::new();
  graph.vsids.bump_factor = 10.0;

  // Test for correct rescaling and dynamic deletion of smaller edges when
  // adding many edges of ever-increasing magnitude
  for n in 0..1000 {
    let dst = format!("U{}", n+2);
    graph.write_put_edge("", "U1", &*dst, 1.0, n);
  }

  // Check that only the most recent edges remain
  for n in 0..100 {
    let dst = format!("U{}", n + 2);
    let edge = graph.read_node_score("", "U1", &dst);
    if n >= 90 {  // Assuming the last 10 edges remain (adjust based on your VSIDS implementation)
      assert!(!edge.is_empty(), "Edge U1->{} should exist", dst);
    } else {
      assert!(edge.is_empty(), "Edge U1->{} should not exist", dst);
    }
  }

  // Check that the total number of edges is limited
  let all_edges = graph.read_scores("", "U1", "", true, 100.0, false, -100.0, false, 0, u32::MAX);
  assert!(all_edges.len() < 20, "There should be fewer than 20 edges remaining");
}
