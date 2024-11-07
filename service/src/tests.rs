use crate::operations::*;
use crate::protocol::*;
use std::time::SystemTime;

fn put_testing_edges(graph : &mut AugMultiGraph, context : &str) {
  graph.write_put_edge(context, "U0cd6bd2dde4f", "B7f628ad203b5",  1.0);
  graph.write_put_edge(context, "U7c9ce0ac22b7", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue5c836f6e6b5", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "C070e739180d6",  9.0);
  graph.write_put_edge(context, "U1c285703fc63", "Bad1c69de7837",  7.0);
  graph.write_put_edge(context, "U25982b736535", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B92e4a185c654",  3.0);
  graph.write_put_edge(context, "U663cd1f1e343", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9a89e0679dec", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B3c467fb437b2", -1.0);
  graph.write_put_edge(context, "U5b09928b977a", "U000000000000",  1.0);
  graph.write_put_edge(context, "U585dfead09c6", "C6d52e861b366", -1.0);
  graph.write_put_edge(context, "U02fbd7c8df4c", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uc1158424318a", "C78d6fac93d00",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "Cbbf2df46955b",  1.0);
  graph.write_put_edge(context, "U4f530cfe771e", "B9c01ce5718d1",  0.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Cd6c9d5cba220",  1.0);
  graph.write_put_edge(context, "Cf4b448ef8618", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U4d6816b2416e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6942e4590e93", "U000000000000",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "Cbbf2df46955b",  4.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B73a44e2bbd44",  1.0);
  graph.write_put_edge(context, "Uab16119974a0", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B5a1c1d3d0140", -1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "U1df3e39ebe59", "Bea16f01b8cc5",  1.0);
  graph.write_put_edge(context, "Uadeb43da4abb", "Bfae1726e4e87",  1.0);
  graph.write_put_edge(context, "C599f6e6f6b64", "U26aca0e369c7",  1.0);
  graph.write_put_edge(context, "Udb60bbb285ca", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf5a84bada7fb", "U000000000000",  1.0);
  graph.write_put_edge(context, "U79466f73dc0c", "B7f628ad203b5",  6.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B3c467fb437b2", -1.0);
  graph.write_put_edge(context, "Uf59dcd0bc354", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ucb84c094edba", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ud7002ae5a86c", "B75a44a52fa29", -2.0);
  graph.write_put_edge(context, "Uc3c31b8a022f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "C6acd550a4ef3", -1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "B5eb4c6be535a",  5.0);
  graph.write_put_edge(context, "B9c01ce5718d1", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U99a0f1f7e6ee", "Bd90a1cf73384",  1.0);
  graph.write_put_edge(context, "U867a75db12ae", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6629a0a8ef04", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub7f9dfb6a7a5", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf31403bd4e20", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0e214fef4f03", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0e6659929c53", "Cffd169930956",  1.0);
  graph.write_put_edge(context, "Cd1c25e32ad21", "Ucd424ac24c15",  1.0);
  graph.write_put_edge(context, "Uac897fe92894", "B9c01ce5718d1", -2.0);
  graph.write_put_edge(context, "Bc4addf09b79f", "U0cd6bd2dde4f",  1.0);
  graph.write_put_edge(context, "U638f5c19326f", "B9cade9992fb9",  1.0);
  graph.write_put_edge(context, "U290a1ab9d54a", "U000000000000",  1.0);
  graph.write_put_edge(context, "U3c63a9b6115a", "Bad1c69de7837",  2.0);
  graph.write_put_edge(context, "U016217c34c6e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "C6acd550a4ef3",  6.0);
  graph.write_put_edge(context, "U99a0f1f7e6ee", "C4d1d582c53c3",  1.0);
  graph.write_put_edge(context, "Be2b46c17f1da", "U80e22da6d8c4",  1.0);
  graph.write_put_edge(context, "B5e7178dd70bb", "Ucbd309d6fcc0",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "U1c285703fc63", -1.0);
  graph.write_put_edge(context, "C4893c40e481d", "Udece0afd9a8b",  1.0);
  graph.write_put_edge(context, "U9e42f6dab85a", "B3c467fb437b2",  1.0);
  graph.write_put_edge(context, "U8842ed397bb7", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue70d59cc8e3f", "B9c01ce5718d1",  1.0);
  graph.write_put_edge(context, "U5c827d7de115", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue94281e36fe8", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Bdf39d0e1daf5", -1.0);
  graph.write_put_edge(context, "U18a178de1dfb", "B70df5dbab8c3",  1.0);
  graph.write_put_edge(context, "U4f530cfe771e", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uad577360d968", "B5eb4c6be535a",  1.0);
  graph.write_put_edge(context, "U526f361717a8", "Cee9901f0f22c",  1.0);
  graph.write_put_edge(context, "C2bbd63b00224", "U80e22da6d8c4",  1.0);
  graph.write_put_edge(context, "Uccc3c7395af6", "U000000000000",  1.0);
  graph.write_put_edge(context, "Cb3c476a45037", "Ue40b938f47a4",  1.0);
  graph.write_put_edge(context, "C22e1102411ce", "U6661263fb410",  1.0);
  graph.write_put_edge(context, "U1c634fdd7c82", "U000000000000",  1.0);
  graph.write_put_edge(context, "U57b6f30fc663", "Bed5126bc655d", -1.0);
  graph.write_put_edge(context, "U6661263fb410", "Cf92f90725ffc",  1.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "C2bbd63b00224",  8.0);
  graph.write_put_edge(context, "U09cf1f359454", "Ba5d64165e5d5", -1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "U79466f73dc0c",  1.0);
  graph.write_put_edge(context, "U40096feaa029", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B5eb4c6be535a", -1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B499bfc56e77b", -1.0);
  graph.write_put_edge(context, "U3c63a9b6115a", "Cf92f90725ffc",  1.0);
  graph.write_put_edge(context, "U9ce5721e93cf", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ud04c89aaf453", "B4f14b223b56d",  1.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "Udece0afd9a8b",  1.0);
  graph.write_put_edge(context, "U38fdca6685ca", "Cf77494dc63d7",  1.0);
  graph.write_put_edge(context, "U83282a51b600", "Be2b46c17f1da",  0.0);
  graph.write_put_edge(context, "U83e829a2e822", "B7f628ad203b5", 14.0);
  graph.write_put_edge(context, "Bc896788cd2ef", "U1bcba4fd7175",  1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "C67e4476fda28",  6.0);
  graph.write_put_edge(context, "C9028c7415403", "Udece0afd9a8b",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "Uadeb43da4abb", "B0e230e9108dd",  4.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "C264c56d501db",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B73a44e2bbd44",  1.0);
  graph.write_put_edge(context, "Ud982a6dee46f", "Be7145faf15cb",  1.0);
  graph.write_put_edge(context, "B0a87a669fc28", "U34252014c05b",  1.0);
  graph.write_put_edge(context, "U0e6659929c53", "Cb967536095de",  1.0);
  graph.write_put_edge(context, "C0f834110f700", "U38fdca6685ca",  1.0);
  graph.write_put_edge(context, "Ucb37b247402a", "U000000000000",  1.0);
  graph.write_put_edge(context, "U72f88cf28226", "Cb11edc3d0bc7",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "C0166be581dd4",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "C6a2263dc469e",  2.0);
  graph.write_put_edge(context, "U526f361717a8", "C52d41a9ad558",  1.0);
  graph.write_put_edge(context, "U5d9b4e4a7baf", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "Cb76829a425d9",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "Cf4b448ef8618",  1.0);
  graph.write_put_edge(context, "U48dcd166b0bd", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uadeb43da4abb", "C30e7409c2d5f",  2.0);
  graph.write_put_edge(context, "U05e4396e2382", "B7f628ad203b5",  1.0);
  graph.write_put_edge(context, "Uf3b5141d73f3", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Cb11edc3d0bc7",  1.0);
  graph.write_put_edge(context, "U18a178de1dfb", "B1533941e2773",  1.0);
  graph.write_put_edge(context, "B506fff6cfc22", "Ub7f9dfb6a7a5",  1.0);
  graph.write_put_edge(context, "Uad577360d968", "C2bbd63b00224",  9.0);
  graph.write_put_edge(context, "U7a8d8324441d", "C4f2dafca724f",  1.0);
  graph.write_put_edge(context, "Uda5b03b660d7", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ucb9952d31a9e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Bd7a8bfcf3337",  1.0);
  graph.write_put_edge(context, "C1ccb4354d684", "Ue202d5b01f8d",  1.0);
  graph.write_put_edge(context, "Ud5b22ebf52f2", "Cd6c9d5cba220",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Ba5d64165e5d5", -1.0);
  graph.write_put_edge(context, "U95f3426b8e5d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0da9e22a248b", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uab20c65d180d", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "C6aebafa4fe8e",  8.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "C588ffef22463",  1.0);
  graph.write_put_edge(context, "Ccae34b3da05e", "Ub93799d9400e",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B9c01ce5718d1",  3.0);
  graph.write_put_edge(context, "Uc35c445325f5", "B75a44a52fa29",  2.0);
  graph.write_put_edge(context, "U362d375c067c", "Ce06bda6030fe",  1.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "Cfdde53c79a2d",  3.0);
  graph.write_put_edge(context, "U09cf1f359454", "B75a44a52fa29",  1.0);
  graph.write_put_edge(context, "Bb5f87c1621d5", "Ub01f4ad1b03f",  1.0);
  graph.write_put_edge(context, "U016217c34c6e", "B3c467fb437b2",  2.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "Udece0afd9a8b",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B63fbe1427d09", -1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "C5782d559baad",  1.0);
  graph.write_put_edge(context, "C3b855f713d19", "U704bd6ecde75",  1.0);
  graph.write_put_edge(context, "U016217c34c6e", "Cb76829a425d9",  2.0);
  graph.write_put_edge(context, "U499f24158a40", "Ba3c4a280657d",  1.0);
  graph.write_put_edge(context, "Ub0205d5d96d0", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0c17798eaab4", "Udece0afd9a8b", -1.0);
  graph.write_put_edge(context, "Ud7002ae5a86c", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uc1158424318a", "Bdf39d0e1daf5",  1.0);
  graph.write_put_edge(context, "U35eb26fc07b4", "U000000000000",  1.0);
  graph.write_put_edge(context, "U96a8bbfce56f", "U000000000000",  1.0);
  graph.write_put_edge(context, "C588ffef22463", "Uef7fbf45ef11",  1.0);
  graph.write_put_edge(context, "U72f88cf28226", "B3f6f837bc345",  1.0);
  graph.write_put_edge(context, "Ba3c4a280657d", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "Ua9d9d5da3948", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Bd90a1cf73384",  3.0);
  graph.write_put_edge(context, "U638f5c19326f", "B9c01ce5718d1",  2.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "U000000000000",  1.0);
  graph.write_put_edge(context, "Udf6d8127c2c6", "U000000000000",  1.0);
  graph.write_put_edge(context, "U362d375c067c", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue072a0d01754", "U000000000000",  1.0);
  graph.write_put_edge(context, "U83282a51b600", "C9462ca240ceb",  1.0);
  graph.write_put_edge(context, "U638f5c19326f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "C54972a5fbc16",  1.0);
  graph.write_put_edge(context, "U3116d27854ab", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub93799d9400e", "B9c01ce5718d1",  5.0);
  graph.write_put_edge(context, "U9e42f6dab85a", "C15d8dfaceb75",  1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "Be2b46c17f1da", -1.0);
  graph.write_put_edge(context, "B8a531802473b", "U016217c34c6e",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "Bb78026d99388",-11.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "C4893c40e481d",  4.0);
  graph.write_put_edge(context, "Cb11edc3d0bc7", "U8a78048d60f7",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Bb1e3630d2f4a",  1.0);
  graph.write_put_edge(context, "U0cd6bd2dde4f", "B92e4a185c654",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B45d72e29f004", -1.0);
  graph.write_put_edge(context, "Cab47a458295f", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "U7dd2b82154e0", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1f348902b446", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uad577360d968", "U000000000000",  1.0);
  graph.write_put_edge(context, "U861750348e9f", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue55b928fa8dd", "Bed5126bc655d",  3.0);
  graph.write_put_edge(context, "U016217c34c6e", "U9a89e0679dec",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "U8a78048d60f7",  1.0);
  graph.write_put_edge(context, "Uc35c445325f5", "U000000000000",  1.0);
  graph.write_put_edge(context, "C6aebafa4fe8e", "U9a2c85753a6d",  1.0);
  graph.write_put_edge(context, "Ucdffb8ab5145", "Cf8fb8c05c116",  1.0);
  graph.write_put_edge(context, "U0cd6bd2dde4f", "B9c01ce5718d1",  1.0);
  graph.write_put_edge(context, "U59abf06369c3", "Cda989f4b466d",  1.0);
  graph.write_put_edge(context, "B4f00e7813add", "U09cf1f359454",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B75a44a52fa29",  3.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "U0c17798eaab4",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "U09cf1f359454",  1.0);
  graph.write_put_edge(context, "U21769235b28d", "C801f204d0da8",  1.0);
  graph.write_put_edge(context, "Uddd01c7863e9", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "B3c467fb437b2",  9.0);
  graph.write_put_edge(context, "U06a4bdf76bf7", "U000000000000",  1.0);
  graph.write_put_edge(context, "U43dcf522b4dd", "B3b3f2ecde430", -1.0);
  graph.write_put_edge(context, "C264c56d501db", "U1bcba4fd7175",  1.0);
  graph.write_put_edge(context, "Ua4041a93bdf4", "B9c01ce5718d1", -1.0);
  graph.write_put_edge(context, "Uc3c31b8a022f", "B45d72e29f004",  3.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "C399b6349ab02",  1.0);
  graph.write_put_edge(context, "Uaaf5341090c6", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "U45578f837ab8", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "B3b3f2ecde430",  3.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "U8a78048d60f7",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B5eb4c6be535a", -1.0);
  graph.write_put_edge(context, "Uc1158424318a", "Cfdde53c79a2d",  6.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "Uadeb43da4abb", -1.0);
  graph.write_put_edge(context, "Ub192fb5e4fee", "U000000000000",  1.0);
  graph.write_put_edge(context, "U7eaa146a4793", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "Bdf39d0e1daf5", -1.0);
  graph.write_put_edge(context, "U96a7841bc98d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U7ac570b5840f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "Cbbf2df46955b",  5.0);
  graph.write_put_edge(context, "U7399d6656581", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ud7186ef65120", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "C78ad459d3b81",  4.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B5a1c1d3d0140", -1.0);
  graph.write_put_edge(context, "U526c52711601", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B25c85fe0df2d", -1.0);
  graph.write_put_edge(context, "Uc1158424318a", "C6acd550a4ef3",  1.0);
  graph.write_put_edge(context, "B310b66ab31fb", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "C4b2b6fd8fa9a",  1.0);
  graph.write_put_edge(context, "B70df5dbab8c3", "U09cf1f359454",  1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "U09cf1f359454",  1.0);
  graph.write_put_edge(context, "U18a178de1dfb", "B75a44a52fa29",  1.0);
  graph.write_put_edge(context, "Uadeb43da4abb", "C9462ca240ceb", -1.0);
  graph.write_put_edge(context, "U9a89e0679dec", "Bb78026d99388",  1.0);
  graph.write_put_edge(context, "U89a6e30efb07", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B491d307dfe01",  3.0);
  graph.write_put_edge(context, "C7c4d9ca4623e", "U8aa2e2623fa5",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "C1c86825bd597",  1.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "C357396896bd0",  1.0);
  graph.write_put_edge(context, "Ub901d5e0edca", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B4f00e7813add",  1.0);
  graph.write_put_edge(context, "U1c285703fc63", "U9e42f6dab85a",  1.0);
  graph.write_put_edge(context, "Ua50dd76e5a75", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1e41b5f3adff", "B310b66ab31fb",  5.0);
  graph.write_put_edge(context, "Cc2b3069cbe5d", "Ub01f4ad1b03f",  1.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "Uadeb43da4abb",  1.0);
  graph.write_put_edge(context, "U9d12c9682206", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ucd424ac24c15", "U000000000000",  1.0);
  graph.write_put_edge(context, "U431131a166be", "U000000000000",  1.0);
  graph.write_put_edge(context, "U59abf06369c3", "B7f628ad203b5",  3.0);
  graph.write_put_edge(context, "U146915ad287e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "B45d72e29f004", -9.0);
  graph.write_put_edge(context, "Ud5f1a29622d1", "U000000000000",  1.0);
  graph.write_put_edge(context, "U05e4396e2382", "Bad1c69de7837", -1.0);
  graph.write_put_edge(context, "Cd795a41fe71d", "U362d375c067c",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "U000000000000",  1.0);
  graph.write_put_edge(context, "U72f88cf28226", "B310b66ab31fb",  1.0);
  graph.write_put_edge(context, "B4f14b223b56d", "Ud04c89aaf453",  1.0);
  graph.write_put_edge(context, "U1e41b5f3adff", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "Uf6ce05bc4e5a", "U000000000000",  1.0);
  graph.write_put_edge(context, "U83e829a2e822", "B0e230e9108dd", -4.0);
  graph.write_put_edge(context, "Ucbd309d6fcc0", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8f0839032839", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "C6a2263dc469e",  3.0);
  graph.write_put_edge(context, "Ueb1e69384e4e", "U000000000000",  1.0);
  graph.write_put_edge(context, "C89c123f7bcf5", "U8842ed397bb7",  1.0);
  graph.write_put_edge(context, "Ufb826ea158e5", "U000000000000",  1.0);
  graph.write_put_edge(context, "U26aca0e369c7", "C4893c40e481d",  2.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "Uaa4e2be7a87a", -1.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "C4893c40e481d", -1.0);
  graph.write_put_edge(context, "U18a178de1dfb", "B3f6f837bc345",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "C25639690ee57",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "Ud9df8116deba",  1.0);
  graph.write_put_edge(context, "Ca8ceac412e6f", "U4ba2e4e81c0e",  1.0);
  graph.write_put_edge(context, "Be29b4af3f7a5", "Uc35c445325f5",  1.0);
  graph.write_put_edge(context, "U1188b2dfb294", "U000000000000",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "U02fbd7c8df4c",  1.0);
  graph.write_put_edge(context, "Cb07d467c1c5e", "U8a78048d60f7",  1.0);
  graph.write_put_edge(context, "U8aa2e2623fa5", "B9c01ce5718d1", -2.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B3b3f2ecde430", -1.0);
  graph.write_put_edge(context, "U88e719e6257d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Cf4b448ef8618",  2.0);
  graph.write_put_edge(context, "U4c619411e5de", "U000000000000",  1.0);
  graph.write_put_edge(context, "C9a2135edf7ff", "U83282a51b600",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B19ea554faf29",  1.0);
  graph.write_put_edge(context, "Ba5d64165e5d5", "U1e41b5f3adff",  1.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ucfe743b8deb1", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9605bd4d1218", "B75a44a52fa29",  4.0);
  graph.write_put_edge(context, "B499bfc56e77b", "Uc1158424318a",  1.0);
  graph.write_put_edge(context, "U1c285703fc63", "Cd59e6cd7e104",  1.0);
  graph.write_put_edge(context, "U83e829a2e822", "Be2b46c17f1da", -8.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B45d72e29f004", -1.0);
  graph.write_put_edge(context, "Cb117f464e558", "U26aca0e369c7",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "U000000000000",  1.0);
  graph.write_put_edge(context, "U4ba2e4e81c0e", "B7f628ad203b5", -2.0);
  graph.write_put_edge(context, "U18a178de1dfb", "B19ea554faf29",  1.0);
  graph.write_put_edge(context, "Cfd59a206c07d", "U99a0f1f7e6ee",  1.0);
  graph.write_put_edge(context, "C8ece5c618ac1", "U21769235b28d",  1.0);
  graph.write_put_edge(context, "Uadeb43da4abb", "Cc9f863ff681b",  2.0);
  graph.write_put_edge(context, "Ubd3c556b8a25", "U000000000000",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "Cdcddfb230cb5",  5.0);
  graph.write_put_edge(context, "Uc1158424318a", "Cc9f863ff681b",  1.0);
  graph.write_put_edge(context, "U26aca0e369c7", "C6acd550a4ef3",  4.0);
  graph.write_put_edge(context, "C8c753f46c014", "U8842ed397bb7",  1.0);
  graph.write_put_edge(context, "C78d6fac93d00", "Uc1158424318a",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "C357396896bd0",  8.0);
  graph.write_put_edge(context, "U389f9f24b31c", "Cd59e6cd7e104",  3.0);
  graph.write_put_edge(context, "Bf3a0a1165271", "U9a89e0679dec",  1.0);
  graph.write_put_edge(context, "U8c33fbcc06d7", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B70df5dbab8c3",  1.0);
  graph.write_put_edge(context, "Cb967536095de", "U0e6659929c53",  1.0);
  graph.write_put_edge(context, "C0b19d314485e", "Uaa4e2be7a87a",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "Bc896788cd2ef", -1.0);
  graph.write_put_edge(context, "Uc35c445325f5", "B9c01ce5718d1",  4.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "B9c01ce5718d1", 10.0);
  graph.write_put_edge(context, "C25639690ee57", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "Ue202d5b01f8d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U362d375c067c", "Bad1c69de7837",  0.0);
  graph.write_put_edge(context, "U1c285703fc63", "C67e4476fda28",  1.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B60d725feca77", -1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Bfefe4e25c870",  3.0);
  graph.write_put_edge(context, "Uc4f728b0d87f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "Cdcddfb230cb5",  4.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "Cb14487d862b3",  1.0);
  graph.write_put_edge(context, "U682c3380036f", "C7986cd8a648a",  1.0);
  graph.write_put_edge(context, "U02fbd7c8df4c", "Bd7a8bfcf3337",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "Cbbf2df46955b",  5.0);
  graph.write_put_edge(context, "U8a78048d60f7", "U6240251593cd",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "C8d80016b8292",  1.0);
  graph.write_put_edge(context, "Uc35c445325f5", "B8a531802473b", -5.0);
  graph.write_put_edge(context, "U704bd6ecde75", "B9c01ce5718d1", -1.0);
  graph.write_put_edge(context, "U77f496546efa", "B9c01ce5718d1", -1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B7f628ad203b5", -1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B10d3f548efc4",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "Cd06fea6a395f",  9.0);
  graph.write_put_edge(context, "U682c3380036f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U36055bb45e5c", "U000000000000",  1.0);
  graph.write_put_edge(context, "U7cdd7999301e", "B7f628ad203b5",  1.0);
  graph.write_put_edge(context, "U526f361717a8", "Cf40e8fb326bc",  1.0);
  graph.write_put_edge(context, "B944097cdd968", "Ue40b938f47a4",  1.0);
  graph.write_put_edge(context, "U946ae258c4b5", "U000000000000",  1.0);
  graph.write_put_edge(context, "U2f08dff8dbdb", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B3f6f837bc345",  1.0);
  graph.write_put_edge(context, "U6661263fb410", "Cc01e00342d63",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "Cb76829a425d9", -1.0);
  graph.write_put_edge(context, "Ccb7dc40f1513", "U6661263fb410",  1.0);
  graph.write_put_edge(context, "U83282a51b600", "C9a2135edf7ff",  1.0);
  graph.write_put_edge(context, "Cb76829a425d9", "Ue7a29d5409f2",  1.0);
  graph.write_put_edge(context, "B45d72e29f004", "U26aca0e369c7",  1.0);
  graph.write_put_edge(context, "Ue6cc7bfa0efd", "B5e7178dd70bb", -7.0);
  graph.write_put_edge(context, "Uac897fe92894", "Be2b46c17f1da",  2.0);
  graph.write_put_edge(context, "B73a44e2bbd44", "U8a78048d60f7",  1.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "C399b6349ab02",  5.0);
  graph.write_put_edge(context, "Cfa08a39f9bb9", "Ubebfe0c8fc29",  1.0);
  graph.write_put_edge(context, "Cdcddfb230cb5", "Udece0afd9a8b",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Bb5f87c1621d5",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "C78d6fac93d00",  2.0);
  graph.write_put_edge(context, "U18a178de1dfb", "U000000000000",  1.0);
  graph.write_put_edge(context, "U2cd96f1b2ea6", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B3c467fb437b2", -1.0);
  graph.write_put_edge(context, "C0cd490b5fb6a", "Uad577360d968",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "Be2b46c17f1da",  1.0);
  graph.write_put_edge(context, "U0f63ee3db59b", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B499bfc56e77b", -1.0);
  graph.write_put_edge(context, "C2cb023b6bcef", "Ucb84c094edba",  1.0);
  graph.write_put_edge(context, "U7a975ca7e0b0", "U000000000000",  1.0);
  graph.write_put_edge(context, "U016217c34c6e", "C4e0db8dec53e",  4.0);
  graph.write_put_edge(context, "Cc931cd2de143", "Ud7002ae5a86c",  1.0);
  graph.write_put_edge(context, "U0c17798eaab4", "C4893c40e481d",  7.0);
  graph.write_put_edge(context, "U1c285703fc63", "U016217c34c6e",  1.0);
  graph.write_put_edge(context, "U4a82930ca419", "U000000000000",  1.0);
  graph.write_put_edge(context, "U682c3380036f", "U6240251593cd",  1.0);
  graph.write_put_edge(context, "U2a62e985bcd5", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1e6a314ef612", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uab766aeb8fd2", "U000000000000",  1.0);
  graph.write_put_edge(context, "U18a178de1dfb", "B4f00e7813add",  1.0);
  graph.write_put_edge(context, "Ud7002ae5a86c", "Cc931cd2de143",  1.0);
  graph.write_put_edge(context, "Uc4ebbce44401", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue5c10787d0db", "U000000000000",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "B79efabc4d8bf",  1.0);
  graph.write_put_edge(context, "U044c5bf57a97", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9605bd4d1218", "U000000000000",  1.0);
  graph.write_put_edge(context, "U99deecf5a281", "U000000000000",  1.0);
  graph.write_put_edge(context, "U3de789cac826", "B9c01ce5718d1",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "C888c86d096d0",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "C10872dc9b863",  1.0);
  graph.write_put_edge(context, "B0e230e9108dd", "U9a89e0679dec",  1.0);
  graph.write_put_edge(context, "U5f148383594f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "C2e31b4b1658f",  1.0);
  graph.write_put_edge(context, "Ufec0de2f341d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B25c85fe0df2d", -1.0);
  graph.write_put_edge(context, "Uc3db248a6e7f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "C81f3f954b643",  1.0);
  graph.write_put_edge(context, "Ued1594827196", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6eab54d64086", "U000000000000",  1.0);
  graph.write_put_edge(context, "C0a576fc389d9", "U1bcba4fd7175",  1.0);
  graph.write_put_edge(context, "U4a6d6f193ae0", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B3f6f837bc345",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "C6d52e861b366",  3.0);
  graph.write_put_edge(context, "U362d375c067c", "Cd795a41fe71d",  1.0);
  graph.write_put_edge(context, "Uc676bd7563ec", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B3b3f2ecde430", -1.0);
  graph.write_put_edge(context, "U585dfead09c6", "B9c01ce5718d1",  2.0);
  graph.write_put_edge(context, "Cfd47f43ac9cf", "U704bd6ecde75",  1.0);
  graph.write_put_edge(context, "U72f88cf28226", "Cd6c9d5cba220",  1.0);
  graph.write_put_edge(context, "Cdd49e516723a", "U704bd6ecde75",  1.0);
  graph.write_put_edge(context, "U26aca0e369c7", "Be2b46c17f1da",  7.0);
  graph.write_put_edge(context, "U6249d53929c4", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uad577360d968", "C588ffef22463", -1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B3c467fb437b2", -1.0);
  graph.write_put_edge(context, "Bf34ee3bfc12b", "U6240251593cd",  1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "Bb78026d99388",  9.0);
  graph.write_put_edge(context, "Ue202d5b01f8d", "B9c01ce5718d1",  2.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B310b66ab31fb",  1.0);
  graph.write_put_edge(context, "U35eb26fc07b4", "C90290100a953",  1.0);
  graph.write_put_edge(context, "Cc9f863ff681b", "Uc1158424318a",  1.0);
  graph.write_put_edge(context, "Uf5ee43a1b729", "C9218f86f6286",  1.0);
  graph.write_put_edge(context, "C888c86d096d0", "U7a8d8324441d",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "Bfefe4e25c870",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "C6f84810d3cd9",  1.0);
  graph.write_put_edge(context, "Cd6c9d5cba220", "Ud5b22ebf52f2",  1.0);
  graph.write_put_edge(context, "U99a0f1f7e6ee", "C96bdee4f11e2",-18.0);
  graph.write_put_edge(context, "U4a82930ca419", "C2d9ab331aed7",  1.0);
  graph.write_put_edge(context, "C4818c4ed20bf", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "Ucd424ac24c15", "Cd1c25e32ad21",  1.0);
  graph.write_put_edge(context, "U585dfead09c6", "U000000000000",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "Bad1c69de7837",  2.0);
  graph.write_put_edge(context, "U20d01ad4d96b", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "Cfdde53c79a2d",  5.0);
  graph.write_put_edge(context, "U5ef3d593e46e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U7382ac807a4f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U5c827d7de115", "B69723edfec8a",  1.0);
  graph.write_put_edge(context, "U88137a4bf483", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "Cd4417a5d718e",  5.0);
  graph.write_put_edge(context, "Ue202d5b01f8d", "C1ccb4354d684",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B9c01ce5718d1",  4.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "Cdcddfb230cb5",  1.0);
  graph.write_put_edge(context, "C81f3f954b643", "U09cf1f359454",  1.0);
  graph.write_put_edge(context, "Ufca294ffe3a5", "U000000000000",  1.0);
  graph.write_put_edge(context, "U02fbd7c8df4c", "B75a44a52fa29",  7.0);
  graph.write_put_edge(context, "U049bf307d470", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1c285703fc63", "C30e7409c2d5f",  4.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Be5bb2f3d56cb", -1.0);
  graph.write_put_edge(context, "U4d82230c274a", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ud719123749e6", "U000000000000",  1.0);
  graph.write_put_edge(context, "B10d3f548efc4", "U99a0f1f7e6ee",  1.0);
  graph.write_put_edge(context, "Uc3c31b8a022f", "B3c467fb437b2", -1.0);
  graph.write_put_edge(context, "C90290100a953", "U35eb26fc07b4",  1.0);
  graph.write_put_edge(context, "Uadeb43da4abb", "U000000000000",  1.0);
  graph.write_put_edge(context, "U18a178de1dfb", "B310b66ab31fb",  1.0);
  graph.write_put_edge(context, "U35eb26fc07b4", "Be2b46c17f1da",  0.0);
  graph.write_put_edge(context, "Ccbd85b8513f3", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "Bc896788cd2ef",  1.0);
  graph.write_put_edge(context, "U5ee57577b2bd", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0cd6bd2dde4f", "C7062e90f7422",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "Be2b46c17f1da", -1.0);
  graph.write_put_edge(context, "C4d1d582c53c3", "U99a0f1f7e6ee",  1.0);
  graph.write_put_edge(context, "U11722d2113bf", "U000000000000",  1.0);
  graph.write_put_edge(context, "U59abf06369c3", "Cb117f464e558", -3.0);
  graph.write_put_edge(context, "B491d307dfe01", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "B25c85fe0df2d", "Uef7fbf45ef11",  1.0);
  graph.write_put_edge(context, "Bdf39d0e1daf5", "Uc1158424318a",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "C3e84102071d1",  6.0);
  graph.write_put_edge(context, "U2371cf61799b", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B63fbe1427d09", -1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Cd5983133fb67",  1.0);
  graph.write_put_edge(context, "Cc616eded7a99", "U0f63ee3db59b",  1.0);
  graph.write_put_edge(context, "U34252014c05b", "B19ea554faf29",  1.0);
  graph.write_put_edge(context, "U6622a635b181", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0f63ee3db59b", "B9c01ce5718d1", -4.0);
  graph.write_put_edge(context, "Uf6ce05bc4e5a", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Cbce32a9b256a",  1.0);
  graph.write_put_edge(context, "U9a89e0679dec", "Bf3a0a1165271",  1.0);
  graph.write_put_edge(context, "U00ace0c36154", "U000000000000",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub1a7f706910f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "B63fbe1427d09", -3.0);
  graph.write_put_edge(context, "U03eaee0e3052", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0cd6bd2dde4f", "Bc4addf09b79f",  1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "B4f00e7813add",  3.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U996b5f6b8bec", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9e42f6dab85a", "Bad1c69de7837",  3.0);
  graph.write_put_edge(context, "U26aca0e369c7", "C599f6e6f6b64",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "Bdf39d0e1daf5", -1.0);
  graph.write_put_edge(context, "Uf8bf10852d43", "B253177f84f08",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "B7f628ad203b5",  1.0);
  graph.write_put_edge(context, "U43dcf522b4dd", "B9c01ce5718d1",  2.0);
  graph.write_put_edge(context, "C13e2a35d917a", "Uf6ce05bc4e5a",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B8a531802473b", -1.0);
  graph.write_put_edge(context, "Uf5ee43a1b729", "U000000000000",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "C247501543b60",  1.0);
  graph.write_put_edge(context, "C2e31b4b1658f", "U8a78048d60f7",  1.0);
  graph.write_put_edge(context, "U3c63a9b6115a", "U000000000000",  1.0);
  graph.write_put_edge(context, "C94bb73c10a06", "Uef7fbf45ef11",  1.0);
  graph.write_put_edge(context, "C357396896bd0", "Udece0afd9a8b",  1.0);
  graph.write_put_edge(context, "C6acd550a4ef3", "Uc1158424318a",  1.0);
  graph.write_put_edge(context, "U016217c34c6e", "C3e84102071d1",  1.0);
  graph.write_put_edge(context, "U18a178de1dfb", "Bc4addf09b79f",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "Cfe90cbd73eab",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "C30e7409c2d5f",  1.0);
  graph.write_put_edge(context, "Uc8bb404462a4", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "Bf3a0a1165271", -1.0);
  graph.write_put_edge(context, "U14a3c81256ab", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uadeb43da4abb", "C2bbd63b00224",  7.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Be2b46c17f1da", -1.0);
  graph.write_put_edge(context, "Caa62fc21e191", "U4ba2e4e81c0e",  1.0);
  graph.write_put_edge(context, "U3f840973f9b5", "U000000000000",  1.0);
  graph.write_put_edge(context, "U02fbd7c8df4c", "Bad1c69de7837", -5.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "B73a44e2bbd44",  3.0);
  graph.write_put_edge(context, "U37f5b0f1e914", "U000000000000",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "U9e42f6dab85a",  1.0);
  graph.write_put_edge(context, "Cb95e21215efa", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "B1533941e2773", "U79466f73dc0c",  1.0);
  graph.write_put_edge(context, "U35108003593e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1e41b5f3adff", "Ba5d64165e5d5",  1.0);
  graph.write_put_edge(context, "U118afa836f11", "U000000000000",  1.0);
  graph.write_put_edge(context, "U682c3380036f", "Bf34ee3bfc12b",  4.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "Uc3c31b8a022f", -1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "U1c285703fc63",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0d47e4861ef0", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1779c42930af", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uc67c60f504ce", "U000000000000",  1.0);
  graph.write_put_edge(context, "U36ddff1a63d8", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6661263fb410", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9ce5721e93cf", "B68247950d9c0",  1.0);
  graph.write_put_edge(context, "Uf8bf10852d43", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8456b2b56820", "U000000000000",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "Uc3c31b8a022f",  1.0);
  graph.write_put_edge(context, "U9a89e0679dec", "Cd06fea6a395f", -1.0);
  graph.write_put_edge(context, "U9e42f6dab85a", "C6a2263dc469e",  5.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B0a87a669fc28",  1.0);
  graph.write_put_edge(context, "Cf40e8fb326bc", "U526f361717a8",  1.0);
  graph.write_put_edge(context, "U4ba2e4e81c0e", "Cb117f464e558",  1.0);
  graph.write_put_edge(context, "U95f3426b8e5d", "B9c01ce5718d1",  2.0);
  graph.write_put_edge(context, "U47b466d57da1", "U000000000000",  1.0);
  graph.write_put_edge(context, "U526f361717a8", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub93799d9400e", "U000000000000",  1.0);
  graph.write_put_edge(context, "C524134905072", "Ucb84c094edba",  1.0);
  graph.write_put_edge(context, "Cd59e6cd7e104", "U80e22da6d8c4",  1.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uc3a2aab8a776", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9a89e0679dec", "Cbbf2df46955b", -1.0);
  graph.write_put_edge(context, "Cbe89905f07d3", "Ub01f4ad1b03f",  1.0);
  graph.write_put_edge(context, "U38fdca6685ca", "U000000000000",  1.0);
  graph.write_put_edge(context, "Bed5126bc655d", "Uc4ebbce44401",  1.0);
  graph.write_put_edge(context, "U9605bd4d1218", "B8a531802473b",  2.0);
  graph.write_put_edge(context, "Ueb139752b907", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub93799d9400e", "B73a44e2bbd44",  5.0);
  graph.write_put_edge(context, "Cee9901f0f22c", "U526f361717a8",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Cc2b3069cbe5d",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "B3b3f2ecde430",  1.0);
  graph.write_put_edge(context, "Ubebfe0c8fc29", "Cfa08a39f9bb9",  1.0);
  graph.write_put_edge(context, "U95f3426b8e5d", "Be7bc0cfecab3",  1.0);
  graph.write_put_edge(context, "C6587e913fbbe", "U6661263fb410",  1.0);
  graph.write_put_edge(context, "U0cd6bd2dde4f", "C5782d559baad",  1.0);
  graph.write_put_edge(context, "U895fd30e1e2a", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B25c85fe0df2d", -1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "U000000000000",  1.0);
  graph.write_put_edge(context, "U016217c34c6e", "B8a531802473b",  1.0);
  graph.write_put_edge(context, "Ccc25a77bfa2a", "U77f496546efa",  1.0);
  graph.write_put_edge(context, "U6240251593cd", "Bf34ee3bfc12b",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "C357396896bd0",  1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "Cb76829a425d9",  8.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "U016217c34c6e",  1.0);
  graph.write_put_edge(context, "Ucdffb8ab5145", "B9c01ce5718d1",  2.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "B75a44a52fa29",  1.0);
  graph.write_put_edge(context, "Cac6ca02355da", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Bc4addf09b79f",  1.0);
  graph.write_put_edge(context, "U831a82104a9e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0c17798eaab4", "U389f9f24b31c",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Ud9df8116deba",  1.0);
  graph.write_put_edge(context, "U0453a921d0e7", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ucd424ac24c15", "B9c01ce5718d1",  2.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "B9c01ce5718d1",  9.0);
  graph.write_put_edge(context, "Ua7759a06a90a", "U000000000000",  1.0);
  graph.write_put_edge(context, "U3c63a9b6115a", "B75a44a52fa29",  5.0);
  graph.write_put_edge(context, "Bea16f01b8cc5", "U1df3e39ebe59",  1.0);
  graph.write_put_edge(context, "U1eafbaaf9536", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "C3fd1fdebe0e9",  9.0);
  graph.write_put_edge(context, "Cffd169930956", "U0e6659929c53",  1.0);
  graph.write_put_edge(context, "U73057a8e8ebf", "U000000000000",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "B3b3f2ecde430", -3.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "C4e0db8dec53e",  1.0);
  graph.write_put_edge(context, "Ua9f1d3f8ee78", "U000000000000",  1.0);
  graph.write_put_edge(context, "U83282a51b600", "B9c01ce5718d1", -1.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "U9e42f6dab85a", -1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "Bfefe4e25c870",  4.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "C070e739180d6",  1.0);
  graph.write_put_edge(context, "C8343a6a576ff", "U02fbd7c8df4c",  1.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "C599f6e6f6b64",  2.0);
  graph.write_put_edge(context, "U77f496546efa", "C9462ca240ceb", -1.0);
  graph.write_put_edge(context, "Cc42c3eeb9d20", "U8a78048d60f7",  1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "Ce1a7d8996eb0", -1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "B70df5dbab8c3",  2.0);
  graph.write_put_edge(context, "Ub10b78df4f63", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "C35678a54ef5f",  1.0);
  graph.write_put_edge(context, "U59abf06369c3", "U000000000000",  1.0);
  graph.write_put_edge(context, "U3de789cac826", "U000000000000",  1.0);
  graph.write_put_edge(context, "U72f88cf28226", "C7722465c957a",  1.0);
  graph.write_put_edge(context, "U9605bd4d1218", "C801f204d0da8",  3.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B10d3f548efc4",  3.0);
  graph.write_put_edge(context, "U02be55e5fdb2", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9a89e0679dec", "U7a8d8324441d",  1.0);
  graph.write_put_edge(context, "Be7145faf15cb", "Ud982a6dee46f",  1.0);
  graph.write_put_edge(context, "U7f5fca21e1e5", "U000000000000",  1.0);
  graph.write_put_edge(context, "Cd06fea6a395f", "Uaa4e2be7a87a",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "C78ad459d3b81",  6.0);
  graph.write_put_edge(context, "Udf0362755172", "U000000000000",  1.0);
  graph.write_put_edge(context, "U28f934dc948e", "U000000000000",  1.0);
  graph.write_put_edge(context, "Bb1e3630d2f4a", "U34252014c05b",  1.0);
  graph.write_put_edge(context, "U6661263fb410", "B75a44a52fa29",  3.0);
  graph.write_put_edge(context, "Uadeb43da4abb", "Bd49e3dac97b0",  1.0);
  graph.write_put_edge(context, "U495c3bb411e1", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub93799d9400e", "Cd4417a5d718e",  1.0);
  graph.write_put_edge(context, "C399b6349ab02", "Uf2b0a6b1d423",  1.0);
  graph.write_put_edge(context, "Ue73fabd3d39a", "U000000000000",  1.0);
  graph.write_put_edge(context, "U4dac6797a9cc", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ua29a81d30ef9", "U000000000000",  1.0);
  graph.write_put_edge(context, "B79efabc4d8bf", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "Ud5b22ebf52f2", "U000000000000",  1.0);
  graph.write_put_edge(context, "U83282a51b600", "C16dfdd8077c8",  1.0);
  graph.write_put_edge(context, "U1c285703fc63", "U9a2c85753a6d",  1.0);
  graph.write_put_edge(context, "B19ea554faf29", "U34252014c05b",  1.0);
  graph.write_put_edge(context, "B75a44a52fa29", "U01814d1ec9ff",  1.0);
  graph.write_put_edge(context, "C35678a54ef5f", "Uaa4e2be7a87a",  1.0);
  graph.write_put_edge(context, "Bc173d5552e2e", "U95f3426b8e5d",  1.0);
  graph.write_put_edge(context, "Uc3c31b8a022f", "Bb78026d99388",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "Be5bb2f3d56cb", -1.0);
  graph.write_put_edge(context, "U4dd243415525", "U000000000000",  1.0);
  graph.write_put_edge(context, "Cb62aea64ea97", "U0e6659929c53",  1.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "B25c85fe0df2d",  1.0);
  graph.write_put_edge(context, "Uefe16d246c36", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B8a531802473b", -1.0);
  graph.write_put_edge(context, "C5127d08eb786", "Ucd424ac24c15",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "Be2b46c17f1da",  5.0);
  graph.write_put_edge(context, "U8a78048d60f7", "U1c285703fc63",  1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "C6a2263dc469e",  1.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "B0e230e9108dd", -1.0);
  graph.write_put_edge(context, "U53eb1f0bdcd2", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Uad577360d968",  1.0);
  graph.write_put_edge(context, "U1afee48387d4", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B4f14b223b56d", -1.0);
  graph.write_put_edge(context, "B3c467fb437b2", "U9e42f6dab85a",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "C30fef1977b4a",  8.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B19ea554faf29",  3.0);
  graph.write_put_edge(context, "U6240251593cd", "B9c01ce5718d1", -4.0);
  graph.write_put_edge(context, "U99a0f1f7e6ee", "C1f41b842849c",  1.0);
  graph.write_put_edge(context, "Uac897fe92894", "Cb117f464e558",  1.0);
  graph.write_put_edge(context, "U704bd6ecde75", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "U0cd6bd2dde4f",  1.0);
  graph.write_put_edge(context, "Ucb84c094edba", "C524134905072",  1.0);
  graph.write_put_edge(context, "B19d70698e3d8", "Uf8bf10852d43",  1.0);
  graph.write_put_edge(context, "Cda989f4b466d", "U59abf06369c3",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B1533941e2773",  3.0);
  graph.write_put_edge(context, "U83e829a2e822", "B5eb4c6be535a",  3.0);
  graph.write_put_edge(context, "U34252014c05b", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ud9df8116deba", "U000000000000",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "Bd7a8bfcf3337",  3.0);
  graph.write_put_edge(context, "Cfdde53c79a2d", "Uef7fbf45ef11",  1.0);
  graph.write_put_edge(context, "U918f8950c4e5", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue6cc7bfa0efd", "B30bf91bf5845",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B3b3f2ecde430", -1.0);
  graph.write_put_edge(context, "C63e21d051dda", "U638f5c19326f",  1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "C30e7409c2d5f",  9.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "C9028c7415403",  3.0);
  graph.write_put_edge(context, "U09cf1f359454", "Bd49e3dac97b0", -1.0);
  graph.write_put_edge(context, "Ua34e02cf30a6", "U000000000000",  1.0);
  graph.write_put_edge(context, "C279db553a831", "U99a0f1f7e6ee",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B45d72e29f004", -1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "B491d307dfe01", -1.0);
  graph.write_put_edge(context, "U99a0f1f7e6ee", "Cfd59a206c07d",  1.0);
  graph.write_put_edge(context, "C52d41a9ad558", "U526f361717a8",  1.0);
  graph.write_put_edge(context, "U7462db3b65c4", "U000000000000",  1.0);
  graph.write_put_edge(context, "U79466f73dc0c", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "Uc3c31b8a022f", -1.0);
  graph.write_put_edge(context, "Uf9ecad50b7e1", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "C0a576fc389d9",  1.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "C94bb73c10a06",  3.0);
  graph.write_put_edge(context, "U3614888a1bdc", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Ud5b22ebf52f2",  1.0);
  graph.write_put_edge(context, "Uf8bf10852d43", "B4115d364e05b",  1.0);
  graph.write_put_edge(context, "U57b6f30fc663", "B30bf91bf5845",  1.0);
  graph.write_put_edge(context, "U72f88cf28226", "U6d2f25cc4264",  0.0);
  graph.write_put_edge(context, "U3c63a9b6115a", "Be5bb2f3d56cb",  1.0);
  graph.write_put_edge(context, "C7722465c957a", "U72f88cf28226",  1.0);
  graph.write_put_edge(context, "Ub7f9dfb6a7a5", "B506fff6cfc22",  1.0);
  graph.write_put_edge(context, "U93bb26f51197", "U000000000000",  1.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "C9028c7415403",  1.0);
  graph.write_put_edge(context, "U79466f73dc0c", "B45d72e29f004",  5.0);
  graph.write_put_edge(context, "U67bf00435429", "U000000000000",  1.0);
  graph.write_put_edge(context, "U3bbfefd5319e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U99a0f1f7e6ee", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B3f6f837bc345",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B4f14b223b56d", -1.0);
  graph.write_put_edge(context, "U6661263fb410", "C31dac67e313b",  1.0);
  graph.write_put_edge(context, "U0e6659929c53", "U000000000000",  1.0);
  graph.write_put_edge(context, "C55a114ca6e7c", "U0e6659929c53",  1.0);
  graph.write_put_edge(context, "C4b2b6fd8fa9a", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "Ue328d7da3b59", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9e42f6dab85a", "C0b19d314485e", -1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "Ba3c4a280657d",  2.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B70df5dbab8c3",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "C30fef1977b4a",  1.0);
  graph.write_put_edge(context, "Uf8eb8562f949", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uc35c445325f5", "Be29b4af3f7a5",  1.0);
  graph.write_put_edge(context, "Uebe87839ab3e", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ud826f91f9025", "U000000000000",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "B5eb4c6be535a",  1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "Cdcddfb230cb5",  3.0);
  graph.write_put_edge(context, "U9605bd4d1218", "Bd7a8bfcf3337",  1.0);
  graph.write_put_edge(context, "U4e7d43caba8f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "B9c01ce5718d1",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "B3b3f2ecde430",  9.0);
  graph.write_put_edge(context, "U83282a51b600", "B45d72e29f004", -1.0);
  graph.write_put_edge(context, "U4db49066d45a", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B60d725feca77", -1.0);
  graph.write_put_edge(context, "U21769235b28d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "Cd59e6cd7e104",  1.0);
  graph.write_put_edge(context, "U26aca0e369c7", "C9028c7415403",  8.0);
  graph.write_put_edge(context, "U161742354fef", "U000000000000",  1.0);
  graph.write_put_edge(context, "B9cade9992fb9", "U638f5c19326f",  1.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "U000000000000",  1.0);
  graph.write_put_edge(context, "U3b6ea55b4098", "U000000000000",  1.0);
  graph.write_put_edge(context, "U05e4396e2382", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B0e230e9108dd", -1.0);
  graph.write_put_edge(context, "U499f24158a40", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "U79466f73dc0c", "Be2b46c17f1da",  4.0);
  graph.write_put_edge(context, "C9218f86f6286", "Uf5ee43a1b729",  1.0);
  graph.write_put_edge(context, "U27847df66cb4", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "Bb78026d99388", -1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "B3b3f2ecde430",  6.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Bd49e3dac97b0", -1.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "C9462ca240ceb",  1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "B0e230e9108dd", -1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "Uf5096f6ab14e",  1.0);
  graph.write_put_edge(context, "U0a5d1c56f5a1", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "Uf2b0a6b1d423",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Bd90a1cf73384",  1.0);
  graph.write_put_edge(context, "Ucb84c094edba", "C2cb023b6bcef",  1.0);
  graph.write_put_edge(context, "Udfbfcd087e6b", "U000000000000",  1.0);
  graph.write_put_edge(context, "U3bf4a5894df1", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ubebfe0c8fc29", "Bfefe4e25c870",  3.0);
  graph.write_put_edge(context, "U9e42f6dab85a", "C070e739180d6",  2.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "C6f84810d3cd9",  1.0);
  graph.write_put_edge(context, "U14a3c81256ab", "B9c01ce5718d1",  0.0);
  graph.write_put_edge(context, "Cd5983133fb67", "U8a78048d60f7",  1.0);
  graph.write_put_edge(context, "U675d1026fe95", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0cd6bd2dde4f", "B75a44a52fa29",  1.0);
  graph.write_put_edge(context, "Ue7a29d5409f2", "Ce1a7d8996eb0",  5.0);
  graph.write_put_edge(context, "Ue40b938f47a4", "B9c01ce5718d1",  0.0);
  graph.write_put_edge(context, "U0e6659929c53", "Cb62aea64ea97",  1.0);
  graph.write_put_edge(context, "U732b06e17fc6", "U000000000000",  1.0);
  graph.write_put_edge(context, "C1c86825bd597", "U01814d1ec9ff",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "Bb78026d99388", -1.0);
  graph.write_put_edge(context, "U393de9ce9ec4", "U000000000000",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "Cbce32a9b256a",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "C96bdee4f11e2",  1.0);
  graph.write_put_edge(context, "Uc76658319bfe", "U000000000000",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "C4893c40e481d",  3.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "B7f628ad203b5",  7.0);
  graph.write_put_edge(context, "Uad577360d968", "Bad1c69de7837",  1.0);
  graph.write_put_edge(context, "U65bb6831c537", "U000000000000",  1.0);
  graph.write_put_edge(context, "U3b78f50182c7", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ud982a6dee46f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "Cdeab5b39cc2a",  1.0);
  graph.write_put_edge(context, "B7f628ad203b5", "U7a8d8324441d",  1.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "C4893c40e481d",  1.0);
  graph.write_put_edge(context, "C96bdee4f11e2", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U0b4010c6af8e", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B0e230e9108dd", -1.0);
  graph.write_put_edge(context, "U02fbd7c8df4c", "C8343a6a576ff",  1.0);
  graph.write_put_edge(context, "C30fef1977b4a", "U7a8d8324441d",  1.0);
  graph.write_put_edge(context, "U77f496546efa", "Ccc25a77bfa2a",  1.0);
  graph.write_put_edge(context, "Uf6ce05bc4e5a", "C13e2a35d917a",  1.0);
  graph.write_put_edge(context, "Ucbb6d026b66f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U07550344f328", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "Cfe90cbd73eab",  1.0);
  graph.write_put_edge(context, "U0c17798eaab4", "B3c467fb437b2",  2.0);
  graph.write_put_edge(context, "Ue6cc7bfa0efd", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue4f003e63773", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9e42f6dab85a", "U80e22da6d8c4",  1.0);
  graph.write_put_edge(context, "U7cdd7999301e", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "Cfdde53c79a2d",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "C94bb73c10a06",  9.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Bb78026d99388", -1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B499bfc56e77b", -1.0);
  graph.write_put_edge(context, "U0c17798eaab4", "Ce1a7d8996eb0",  6.0);
  graph.write_put_edge(context, "C7062e90f7422", "U01814d1ec9ff",  1.0);
  graph.write_put_edge(context, "Cf8fb8c05c116", "Ucdffb8ab5145",  1.0);
  graph.write_put_edge(context, "B60d725feca77", "U80e22da6d8c4",  1.0);
  graph.write_put_edge(context, "C070e739180d6", "U80e22da6d8c4",  1.0);
  graph.write_put_edge(context, "C3e84102071d1", "U016217c34c6e",  1.0);
  graph.write_put_edge(context, "B69723edfec8a", "U5c827d7de115",  1.0);
  graph.write_put_edge(context, "U0c17798eaab4", "B0e230e9108dd",  3.0);
  graph.write_put_edge(context, "U0c17798eaab4", "C4e0db8dec53e",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "Cfc639b9aa3e0",  1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uad577360d968", "C0cd490b5fb6a",  1.0);
  graph.write_put_edge(context, "Uc3c31b8a022f", "U1c285703fc63",  1.0);
  graph.write_put_edge(context, "Ucc8ea98c2b41", "U000000000000",  1.0);
  graph.write_put_edge(context, "U3c63a9b6115a", "B9c01ce5718d1",  3.0);
  graph.write_put_edge(context, "B3b3f2ecde430", "U7a8d8324441d",  1.0);
  graph.write_put_edge(context, "U38fdca6685ca", "B9c01ce5718d1",  0.0);
  graph.write_put_edge(context, "Ua1ca6a97ea28", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "C78ad459d3b81",  1.0);
  graph.write_put_edge(context, "Uc1158424318a", "C67e4476fda28", -1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "U01814d1ec9ff",  1.0);
  graph.write_put_edge(context, "Ua5a9eab9732d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U4ba2e4e81c0e", "Ca8ceac412e6f",  1.0);
  graph.write_put_edge(context, "Cfe90cbd73eab", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U79466f73dc0c", "B1533941e2773",  1.0);
  graph.write_put_edge(context, "C8d80016b8292", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "Uf6ce05bc4e5a", "Bf843e315d71b",  1.0);
  graph.write_put_edge(context, "U6661263fb410", "C6587e913fbbe",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Bb1e3630d2f4a",  3.0);
  graph.write_put_edge(context, "Ucd424ac24c15", "C5127d08eb786",  1.0);
  graph.write_put_edge(context, "Uc1158424318a", "C4e0db8dec53e",  4.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "Bad1c69de7837", -1.0);
  graph.write_put_edge(context, "Ud7002ae5a86c", "C7a807e462b65",  1.0);
  graph.write_put_edge(context, "C3c17b70c3357", "U3de789cac826",  1.0);
  graph.write_put_edge(context, "U5cfee124371b", "U000000000000",  1.0);
  graph.write_put_edge(context, "Bd90a1cf73384", "U99a0f1f7e6ee",  1.0);
  graph.write_put_edge(context, "Uc3a349f521e1", "U000000000000",  1.0);
  graph.write_put_edge(context, "U83282a51b600", "B7f628ad203b5",  1.0);
  graph.write_put_edge(context, "U0cd6bd2dde4f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U26aca0e369c7", "Cb117f464e558",  1.0);
  graph.write_put_edge(context, "Ce1a7d8996eb0", "Uf5096f6ab14e",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "Bad1c69de7837", -1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B25c85fe0df2d", -1.0);
  graph.write_put_edge(context, "U95f3426b8e5d", "Bc173d5552e2e",  1.0);
  graph.write_put_edge(context, "U9a89e0679dec", "C6aebafa4fe8e",  8.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "B8a531802473b",  8.0);
  graph.write_put_edge(context, "Ub93799d9400e", "B75a44a52fa29",  5.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Bf34ee3bfc12b",  1.0);
  graph.write_put_edge(context, "U34252014c05b", "B0a87a669fc28",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Cbe89905f07d3",  1.0);
  graph.write_put_edge(context, "Ud18285ef1202", "U000000000000",  1.0);
  graph.write_put_edge(context, "U7c88b933c58d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U4ba2e4e81c0e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "U7a8d8324441d",  1.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "Ce1a7d8996eb0",  1.0);
  graph.write_put_edge(context, "U8889e390d38b", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9e972ae23870", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B4f00e7813add",  1.0);
  graph.write_put_edge(context, "Bd7a8bfcf3337", "U02fbd7c8df4c",  1.0);
  graph.write_put_edge(context, "C2d9ab331aed7", "U4a82930ca419",  1.0);
  graph.write_put_edge(context, "C247501543b60", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "Cfdde53c79a2d",  4.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "C613f00c1333c",  1.0);
  graph.write_put_edge(context, "C31dac67e313b", "U6661263fb410",  1.0);
  graph.write_put_edge(context, "U864ef33f7249", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf91b831f1eb7", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ua85bc934db95", "U000000000000",  1.0);
  graph.write_put_edge(context, "C67e4476fda28", "U1c285703fc63",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "U000000000000",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "C3e84102071d1",  4.0);
  graph.write_put_edge(context, "U9605bd4d1218", "Cab47a458295f",  3.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "C992d8370db6b",  1.0);
  graph.write_put_edge(context, "Cbce32a9b256a", "U389f9f24b31c",  1.0);
  graph.write_put_edge(context, "Uc1158424318a", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B63fbe1427d09", -1.0);
  graph.write_put_edge(context, "B63fbe1427d09", "U1c285703fc63",  1.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "B60d725feca77",  8.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "Bfefe4e25c870",  5.0);
  graph.write_put_edge(context, "U09cf1f359454", "B9c01ce5718d1",  2.0);
  graph.write_put_edge(context, "U83282a51b600", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uac897fe92894", "B7f628ad203b5",  1.0);
  graph.write_put_edge(context, "B8120aa1edccb", "Ue40b938f47a4",  1.0);
  graph.write_put_edge(context, "Ubbe66e390603", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ueb139752b907", "U79466f73dc0c",  1.0);
  graph.write_put_edge(context, "U1c285703fc63", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0e6659929c53", "B9c01ce5718d1",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B491d307dfe01",  2.0);
  graph.write_put_edge(context, "U18a178de1dfb", "B73a44e2bbd44",  1.0);
  graph.write_put_edge(context, "U38fdca6685ca", "C958e7588ae1c",  1.0);
  graph.write_put_edge(context, "U72f88cf28226", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue70d59cc8e3f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U35eb26fc07b4", "B60d725feca77",  1.0);
  graph.write_put_edge(context, "Cdeab5b39cc2a", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U0f63ee3db59b", "Cbcf72c7e6061",  1.0);
  graph.write_put_edge(context, "C4e0db8dec53e", "U0c17798eaab4",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B7f628ad203b5", -1.0);
  graph.write_put_edge(context, "Uc78a29f47b21", "U000000000000",  1.0);
  graph.write_put_edge(context, "C472b59eeafa5", "U4a82930ca419",  1.0);
  graph.write_put_edge(context, "U2c8e7b806cb4", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "C247501543b60",  1.0);
  graph.write_put_edge(context, "Cfc639b9aa3e0", "U389f9f24b31c",  1.0);
  graph.write_put_edge(context, "U95f3426b8e5d", "B23b74174e659",  1.0);
  graph.write_put_edge(context, "U99deecf5a281", "B9c01ce5718d1",  1.0);
  graph.write_put_edge(context, "Bfefe4e25c870", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "Ua12e78308f49", "B75a44a52fa29",  4.0);
  graph.write_put_edge(context, "U1d5b8c2a3400", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ucdffb8ab5145", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "Cd06fea6a395f",  1.0);
  graph.write_put_edge(context, "U682c3380036f", "B75a44a52fa29",  2.0);
  graph.write_put_edge(context, "Ue202d5b01f8d", "C637133747308",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "Cab47a458295f",  1.0);
  graph.write_put_edge(context, "U0c17798eaab4", "Cd06fea6a395f",  8.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Bd7a8bfcf3337",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "U01814d1ec9ff",  1.0);
  graph.write_put_edge(context, "Ua01529fb0d57", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf5ee43a1b729", "B47cc49866c37",  1.0);
  graph.write_put_edge(context, "U611323f9392c", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uac897fe92894", "C9462ca240ceb",  0.0);
  graph.write_put_edge(context, "U21769235b28d", "C481cd737c873",  1.0);
  graph.write_put_edge(context, "C6f84810d3cd9", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "Ub93c197b25c5", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uc3c31b8a022f", "C78d6fac93d00",  3.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "C4f2dafca724f",  8.0);
  graph.write_put_edge(context, "U9361426a2e51", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "B0e230e9108dd",  2.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Bb78026d99388", -1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Cdcddfb230cb5",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "U0cd6bd2dde4f",  1.0);
  graph.write_put_edge(context, "Ud5b22ebf52f2", "B310b66ab31fb",  1.0);
  graph.write_put_edge(context, "U802de6b3675a", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B0e230e9108dd", -1.0);
  graph.write_put_edge(context, "U09cf1f359454", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "Uad577360d968", "Cbce32a9b256a",  3.0);
  graph.write_put_edge(context, "U09cf1f359454", "Be29b4af3f7a5", -1.0);
  graph.write_put_edge(context, "U5e1dd853cab5", "U000000000000",  1.0);
  graph.write_put_edge(context, "C958e7588ae1c", "U38fdca6685ca",  1.0);
  graph.write_put_edge(context, "Ub152bb6d4a86", "U000000000000",  1.0);
  graph.write_put_edge(context, "B23b74174e659", "U95f3426b8e5d",  1.0);
  graph.write_put_edge(context, "U47b466d57da1", "Bad1c69de7837", -3.0);
  graph.write_put_edge(context, "U016217c34c6e", "Ca0a6aea6c82e",  1.0);
  graph.write_put_edge(context, "U18a178de1dfb", "B491d307dfe01",  1.0);
  graph.write_put_edge(context, "U79466f73dc0c", "B9c01ce5718d1", -6.0);
  graph.write_put_edge(context, "Ucfb9f0586d9e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U704bd6ecde75", "C3b855f713d19",  1.0);
  graph.write_put_edge(context, "Uc1158424318a", "C0b19d314485e",  4.0);
  graph.write_put_edge(context, "U1c285703fc63", "Uad577360d968",  1.0);
  graph.write_put_edge(context, "Ce49159fe9d01", "U6661263fb410",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B8a531802473b", -1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B310b66ab31fb",  4.0);
  graph.write_put_edge(context, "Ub22f9ca70b59", "U000000000000",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "B491d307dfe01",  1.0);
  graph.write_put_edge(context, "Ue40b938f47a4", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ud04c89aaf453", "U8a78048d60f7",  1.0);
  graph.write_put_edge(context, "U9605bd4d1218", "B5a1c1d3d0140",  2.0);
  graph.write_put_edge(context, "U2cb58c48703b", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ud04c89aaf453", "B73a44e2bbd44",  4.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B9c01ce5718d1",  1.0);
  graph.write_put_edge(context, "Uc4ebbce44401", "Bed5126bc655d",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B79efabc4d8bf",  1.0);
  graph.write_put_edge(context, "Be5bb2f3d56cb", "U3c63a9b6115a",  1.0);
  graph.write_put_edge(context, "U8842ed397bb7", "C89c123f7bcf5",  1.0);
  graph.write_put_edge(context, "Uceaf0448e060", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uc1158424318a", "B7f628ad203b5",  8.0);
  graph.write_put_edge(context, "U0f63ee3db59b", "Cc616eded7a99",  1.0);
  graph.write_put_edge(context, "U26aca0e369c7", "B45d72e29f004",  1.0);
  graph.write_put_edge(context, "Ubeded808a9c0", "B9c01ce5718d1",  6.0);
  graph.write_put_edge(context, "B30bf91bf5845", "Ue6cc7bfa0efd",  1.0);
  graph.write_put_edge(context, "U005d51b8771c", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ubcf610883f95", "U000000000000",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "Cb95e21215efa",  1.0);
  graph.write_put_edge(context, "C16dfdd8077c8", "U83282a51b600",  1.0);
  graph.write_put_edge(context, "U606a687682ec", "U000000000000",  1.0);
  graph.write_put_edge(context, "C1f41b842849c", "U99a0f1f7e6ee",  1.0);
  graph.write_put_edge(context, "Ue20d37fe1d62", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1e5391821528", "U000000000000",  1.0);
  graph.write_put_edge(context, "U1c285703fc63", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B60d725feca77", -1.0);
  graph.write_put_edge(context, "B3f6f837bc345", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "U704bd6ecde75", "Cfd47f43ac9cf",  1.0);
  graph.write_put_edge(context, "U4a82930ca419", "C472b59eeafa5",  1.0);
  graph.write_put_edge(context, "Ua12e78308f49", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Ub93799d9400e",  1.0);
  graph.write_put_edge(context, "B47cc49866c37", "Uf5ee43a1b729",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B491d307dfe01",  3.0);
  graph.write_put_edge(context, "U7a8d8324441d", "C3fd1fdebe0e9",  1.0);
  graph.write_put_edge(context, "U0c17798eaab4", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0e6659929c53", "C55a114ca6e7c",  1.0);
  graph.write_put_edge(context, "U0ff6902d8945", "U000000000000",  1.0);
  graph.write_put_edge(context, "U7a8d8324441d", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "U638f5c19326f", "C63e21d051dda",  1.0);
  graph.write_put_edge(context, "U8ec514590d15", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6240251593cd", "B75a44a52fa29",  4.0);
  graph.write_put_edge(context, "C7986cd8a648a", "U682c3380036f",  1.0);
  graph.write_put_edge(context, "C637133747308", "Ue202d5b01f8d",  1.0);
  graph.write_put_edge(context, "U9605bd4d1218", "B9c01ce5718d1",  2.0);
  graph.write_put_edge(context, "B68247950d9c0", "U9ce5721e93cf",  1.0);
  graph.write_put_edge(context, "Bf843e315d71b", "Uf6ce05bc4e5a",  1.0);
  graph.write_put_edge(context, "U5f8c0e9c8cc4", "U000000000000",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "B5a1c1d3d0140",  5.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B5eb4c6be535a", -1.0);
  graph.write_put_edge(context, "Ubebfe0c8fc29", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B79efabc4d8bf",  2.0);
  graph.write_put_edge(context, "U9c1051c9bb99", "U000000000000",  1.0);
  graph.write_put_edge(context, "B4115d364e05b", "Uf8bf10852d43",  1.0);
  graph.write_put_edge(context, "Ue40b938f47a4", "B8120aa1edccb",  1.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "Uc1158424318a", "B499bfc56e77b",  1.0);
  graph.write_put_edge(context, "U4ff50cbb890f", "U000000000000",  1.0);
  graph.write_put_edge(context, "U660f0dfe3117", "U000000000000",  1.0);
  graph.write_put_edge(context, "U26aca0e369c7", "U000000000000",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "Cd172fb3fdc41",  1.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "C0b19d314485e",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "B5eb4c6be535a", -1.0);
  graph.write_put_edge(context, "U17789c126682", "U000000000000",  1.0);
  graph.write_put_edge(context, "Bad1c69de7837", "Uad577360d968",  1.0);
  graph.write_put_edge(context, "Ubd48a3c8df1e", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Bf34ee3bfc12b",  3.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "C2bbd63b00224",  1.0);
  graph.write_put_edge(context, "U35eb26fc07b4", "Cb117f464e558", -1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Cc42c3eeb9d20",  1.0);
  graph.write_put_edge(context, "C78ad459d3b81", "U9a2c85753a6d",  1.0);
  graph.write_put_edge(context, "U57a6591c7ee1", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8aa2e2623fa5", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf2b0a6b1d423", "C3fd1fdebe0e9",  7.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "B7f628ad203b5",  9.0);
  graph.write_put_edge(context, "U8b70c7c00136", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf75d4cbe5430", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Be2b46c17f1da", -1.0);
  graph.write_put_edge(context, "U84f274f30e33", "U000000000000",  1.0);
  graph.write_put_edge(context, "U26451935eec8", "U000000000000",  1.0);
  graph.write_put_edge(context, "U83e829a2e822", "Bad1c69de7837", -4.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "C35678a54ef5f",  5.0);
  graph.write_put_edge(context, "U5d0cd6daa146", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "U7a8d8324441d",  1.0);
  graph.write_put_edge(context, "C9462ca240ceb", "Uf5096f6ab14e",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "C4893c40e481d", -1.0);
  graph.write_put_edge(context, "U18a178de1dfb", "Bf34ee3bfc12b",  1.0);
  graph.write_put_edge(context, "Bfae1726e4e87", "Uadeb43da4abb",  1.0);
  graph.write_put_edge(context, "U3de789cac826", "C3c17b70c3357",  1.0);
  graph.write_put_edge(context, "U6661263fb410", "Ce49159fe9d01",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B310b66ab31fb",  1.0);
  graph.write_put_edge(context, "Ua4041a93bdf4", "U000000000000",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "C6aebafa4fe8e",  6.0);
  graph.write_put_edge(context, "U21769235b28d", "C6d52e861b366",  1.0);
  graph.write_put_edge(context, "Uad577360d968", "C6a2263dc469e",  5.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "Bad1c69de7837",  9.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "C6aebafa4fe8e",  1.0);
  graph.write_put_edge(context, "U1e41b5f3adff", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ueb139752b907", "B1533941e2773",  1.0);
  graph.write_put_edge(context, "Udece0afd9a8b", "U1c285703fc63",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "Be2b46c17f1da", -1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B92e4a185c654",  1.0);
  graph.write_put_edge(context, "Cbcf72c7e6061", "U0f63ee3db59b",  1.0);
  graph.write_put_edge(context, "Cf92f90725ffc", "U6661263fb410",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6106ae1092fa", "U000000000000",  1.0);
  graph.write_put_edge(context, "U037b51a34f3c", "U000000000000",  1.0);
  graph.write_put_edge(context, "U95f3426b8e5d", "B79efabc4d8bf",  3.0);
  graph.write_put_edge(context, "U526f361717a8", "B9c01ce5718d1",  0.0);
  graph.write_put_edge(context, "Ud9df8116deba", "B310b66ab31fb",  1.0);
  graph.write_put_edge(context, "U6661263fb410", "C22e1102411ce",  1.0);
  graph.write_put_edge(context, "Udc7c82928598", "U000000000000",  1.0);
  graph.write_put_edge(context, "U22ad914a7065", "U000000000000",  1.0);
  graph.write_put_edge(context, "U21769235b28d", "C8ece5c618ac1",  1.0);
  graph.write_put_edge(context, "U99a0f1f7e6ee", "B10d3f548efc4",  1.0);
  graph.write_put_edge(context, "Ce06bda6030fe", "U362d375c067c",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Bf3a0a1165271", -1.0);
  graph.write_put_edge(context, "Uc44834086c03", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue55b928fa8dd", "U000000000000",  1.0);
  graph.write_put_edge(context, "C5167c9b3d347", "U362d375c067c",  1.0);
  graph.write_put_edge(context, "U7a54f2f24cf6", "U000000000000",  1.0);
  graph.write_put_edge(context, "C613f00c1333c", "U80e22da6d8c4",  1.0);
  graph.write_put_edge(context, "U6727ddef0614", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ubeded808a9c0", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B0a87a669fc28",  3.0);
  graph.write_put_edge(context, "U6240251593cd", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "C588ffef22463",  4.0);
  graph.write_put_edge(context, "Uc1158424318a", "C9028c7415403", -1.0);
  graph.write_put_edge(context, "Ue40b938f47a4", "B944097cdd968",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B310b66ab31fb",  1.0);
  graph.write_put_edge(context, "U016217c34c6e", "U80e22da6d8c4",  1.0);
  graph.write_put_edge(context, "U362d375c067c", "C5060d0101429",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "Ce1a7d8996eb0",  2.0);
  graph.write_put_edge(context, "U43dcf522b4dd", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uad577360d968", "C399b6349ab02",  6.0);
  graph.write_put_edge(context, "C992d8370db6b", "U6d2f25cc4264",  1.0);
  graph.write_put_edge(context, "Ud2c791d9e879", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf6ce05bc4e5a", "B9c01ce5718d1",  1.0);
  graph.write_put_edge(context, "U016217c34c6e", "C15d8dfaceb75",  8.0);
  graph.write_put_edge(context, "Ub93799d9400e", "B491d307dfe01",  2.0);
  graph.write_put_edge(context, "Ubd9c1e76bb53", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "Cac6ca02355da",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "C4f2dafca724f",  5.0);
  graph.write_put_edge(context, "Ua6dfa92ad74d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0c17798eaab4", "Uad577360d968",  1.0);
  graph.write_put_edge(context, "U8842ed397bb7", "C8c753f46c014",  1.0);
  graph.write_put_edge(context, "U0667457dabfe", "U000000000000",  1.0);
  graph.write_put_edge(context, "U09cf1f359454", "B73a44e2bbd44",  1.0);
  graph.write_put_edge(context, "U6d2f25cc4264", "C8d80016b8292",  1.0);
  graph.write_put_edge(context, "Ufa76b4bb3c95", "U000000000000",  1.0);
  graph.write_put_edge(context, "C7a807e462b65", "Ud7002ae5a86c",  1.0);
  graph.write_put_edge(context, "C481cd737c873", "U21769235b28d",  1.0);
  graph.write_put_edge(context, "Ub786ef7c9e9f", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf3b5141d73f3", "B9c01ce5718d1", -3.0);
  graph.write_put_edge(context, "U430a8328643b", "U000000000000",  1.0);
  graph.write_put_edge(context, "U72f88cf28226", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "Bd49e3dac97b0", "Uadeb43da4abb",  1.0);
  graph.write_put_edge(context, "C0166be581dd4", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U83e829a2e822", "U000000000000",  1.0);
  graph.write_put_edge(context, "Cd172fb3fdc41", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U79466f73dc0c", "U01814d1ec9ff",  1.0);
  graph.write_put_edge(context, "U362d375c067c", "C5167c9b3d347",  1.0);
  graph.write_put_edge(context, "Ue6cc7bfa0efd", "Bed5126bc655d",  7.0);
  graph.write_put_edge(context, "U8842ed397bb7", "C789dceb76123",  1.0);
  graph.write_put_edge(context, "Uda0a7acaeb90", "U000000000000",  1.0);
  graph.write_put_edge(context, "U499f24158a40", "Ccbd85b8513f3",  1.0);
  graph.write_put_edge(context, "U0a227036e790", "U000000000000",  1.0);
  graph.write_put_edge(context, "Cf77494dc63d7", "U38fdca6685ca",  1.0);
  graph.write_put_edge(context, "U9a89e0679dec", "B0e230e9108dd",  1.0);
  graph.write_put_edge(context, "Cd4417a5d718e", "Ub93799d9400e",  1.0);
  graph.write_put_edge(context, "U7553cc7bb536", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "Ud9df8116deba",  1.0);
  graph.write_put_edge(context, "U5f7ff9cb9304", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uee0fbe261b7f", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "U000000000000",  1.0);
  graph.write_put_edge(context, "Cb14487d862b3", "Uf5096f6ab14e",  1.0);
  graph.write_put_edge(context, "Ud04c89aaf453", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "C588ffef22463",  1.0);
  graph.write_put_edge(context, "U0c17798eaab4", "C588ffef22463",  5.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "C78d6fac93d00", -1.0);
  graph.write_put_edge(context, "Ue3b747447a90", "U000000000000",  1.0);
  graph.write_put_edge(context, "U59abf06369c3", "Be2b46c17f1da", -1.0);
  graph.write_put_edge(context, "Ucb84c094edba", "B491d307dfe01",  0.0);
  graph.write_put_edge(context, "Uc2bfe7e7308d", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ubeded808a9c0", "B7f628ad203b5", -9.0);
  graph.write_put_edge(context, "Uac897fe92894", "U000000000000",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "Ue7a29d5409f2",  1.0);
  graph.write_put_edge(context, "U4ba2e4e81c0e", "Caa62fc21e191",  1.0);
  graph.write_put_edge(context, "Ub93799d9400e", "Ccae34b3da05e",  1.0);
  graph.write_put_edge(context, "U9e42f6dab85a", "U000000000000",  1.0);
  graph.write_put_edge(context, "U0e6659929c53", "C6d52e861b366", -1.0);
  graph.write_put_edge(context, "U38fdca6685ca", "C0f834110f700",  1.0);
  graph.write_put_edge(context, "B92e4a185c654", "U41784ed376c3",  1.0);
  graph.write_put_edge(context, "B5a1c1d3d0140", "Uc3c31b8a022f",  1.0);
  graph.write_put_edge(context, "C6a2263dc469e", "Uf2b0a6b1d423",  1.0);
  graph.write_put_edge(context, "U9a89e0679dec", "Cbce32a9b256a",  6.0);
  graph.write_put_edge(context, "Uf5096f6ab14e", "C3e84102071d1",  1.0);
  graph.write_put_edge(context, "Uef7fbf45ef11", "C94bb73c10a06",  1.0);
  graph.write_put_edge(context, "C4f2dafca724f", "U7a8d8324441d",  1.0);
  graph.write_put_edge(context, "U4f530cfe771e", "B7f628ad203b5",  0.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B75a44a52fa29",  1.0);
  graph.write_put_edge(context, "Ubd93205079e9", "U000000000000",  1.0);
  graph.write_put_edge(context, "U9a2c85753a6d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U7bd2e29031a4", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ud5f1a29622d1", "B7f628ad203b5",  1.0);
  graph.write_put_edge(context, "Cbbf2df46955b", "U7a8d8324441d",  1.0);
  graph.write_put_edge(context, "U8676859527f3", "U000000000000",  1.0);
  graph.write_put_edge(context, "B5eb4c6be535a", "Uad577360d968",  1.0);
  graph.write_put_edge(context, "U95f3426b8e5d", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U01814d1ec9ff", "C7062e90f7422",  1.0);
  graph.write_put_edge(context, "U41784ed376c3", "U000000000000",  1.0);
  graph.write_put_edge(context, "U99a0f1f7e6ee", "C279db553a831",  1.0);
  graph.write_put_edge(context, "C15d8dfaceb75", "U9e42f6dab85a",  1.0);
  graph.write_put_edge(context, "Ca0a6aea6c82e", "U016217c34c6e",  1.0);
  graph.write_put_edge(context, "Uc3c31b8a022f", "B5a1c1d3d0140",  1.0);
  graph.write_put_edge(context, "U35eb26fc07b4", "B7f628ad203b5", -2.0);
  graph.write_put_edge(context, "U5f2702cc8ade", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ue40b938f47a4", "Cb3c476a45037",  1.0);
  graph.write_put_edge(context, "U11456af7d414", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uaa4e2be7a87a", "C070e739180d6",  8.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B8a531802473b", -1.0);
  graph.write_put_edge(context, "U1df3e39ebe59", "U000000000000",  1.0);
  graph.write_put_edge(context, "U6661263fb410", "Ccb7dc40f1513",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Cb07d467c1c5e",  1.0);
  graph.write_put_edge(context, "C789dceb76123", "U8842ed397bb7",  1.0);
  graph.write_put_edge(context, "Uad577360d968", "U389f9f24b31c",  1.0);
  graph.write_put_edge(context, "U57b6f30fc663", "U000000000000",  1.0);
  graph.write_put_edge(context, "C54972a5fbc16", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U2d8ff859cca4", "U000000000000",  1.0);
  graph.write_put_edge(context, "Be7bc0cfecab3", "U95f3426b8e5d",  1.0);
  graph.write_put_edge(context, "U052641f28245", "U000000000000",  1.0);
  graph.write_put_edge(context, "Bb78026d99388", "U9a89e0679dec",  1.0);
  graph.write_put_edge(context, "U389f9f24b31c", "B25c85fe0df2d",  5.0);
  graph.write_put_edge(context, "Uc244d6132650", "U000000000000",  1.0);
  graph.write_put_edge(context, "U79466f73dc0c", "Bad1c69de7837",  2.0);
  graph.write_put_edge(context, "U622a649ddf56", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Ba3c4a280657d",  3.0);
  graph.write_put_edge(context, "C6d52e861b366", "U21769235b28d",  1.0);
  graph.write_put_edge(context, "U1eedef3e4d10", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "C6acd550a4ef3",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "B60d725feca77",  1.0);
  graph.write_put_edge(context, "U1c285703fc63", "B63fbe1427d09",  1.0);
  graph.write_put_edge(context, "Uc5d62a177997", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8aa2e2623fa5", "C7c4d9ca4623e",  1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "C6d52e861b366", -1.0);
  graph.write_put_edge(context, "C30e7409c2d5f", "U80e22da6d8c4",  1.0);
  graph.write_put_edge(context, "Ub01f4ad1b03f", "B491d307dfe01",  1.0);
  graph.write_put_edge(context, "U77f496546efa", "U000000000000",  1.0);
  graph.write_put_edge(context, "C801f204d0da8", "U21769235b28d",  1.0);
  graph.write_put_edge(context, "U798f0a5b78f0", "U000000000000",  1.0);
  graph.write_put_edge(context, "C5782d559baad", "U0cd6bd2dde4f",  1.0);
  graph.write_put_edge(context, "U41784ed376c3", "B92e4a185c654",  1.0);
  graph.write_put_edge(context, "U26aca0e369c7", "Cb117f464e558",  6.0);
  graph.write_put_edge(context, "U704bd6ecde75", "Cdd49e516723a",  1.0);
  graph.write_put_edge(context, "Ucbca544d500f", "U000000000000",  1.0);
  graph.write_put_edge(context, "Ucbd309d6fcc0", "B5e7178dd70bb",  1.0);
  graph.write_put_edge(context, "Ue2570414501b", "U000000000000",  1.0);
  graph.write_put_edge(context, "Uf8bf10852d43", "B19d70698e3d8",  1.0);
  graph.write_put_edge(context, "U5502925dfe14", "U000000000000",  1.0);
  graph.write_put_edge(context, "U8fc7861a79b9", "U000000000000",  1.0);
  graph.write_put_edge(context, "C5060d0101429", "U362d375c067c",  1.0);
  graph.write_put_edge(context, "B253177f84f08", "Uf8bf10852d43",  1.0);
  graph.write_put_edge(context, "U34252014c05b", "Bb1e3630d2f4a",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "Cb14487d862b3",  6.0);
  graph.write_put_edge(context, "U707f9ed34910", "U000000000000",  1.0);
  graph.write_put_edge(context, "Cc01e00342d63", "U6661263fb410",  1.0);
  graph.write_put_edge(context, "C10872dc9b863", "U499f24158a40",  1.0);
  graph.write_put_edge(context, "U8a78048d60f7", "Be29b4af3f7a5", -1.0);
  graph.write_put_edge(context, "U499f24158a40", "C4818c4ed20bf",  1.0);
  graph.write_put_edge(context, "C3fd1fdebe0e9", "U7a8d8324441d",  1.0);
  graph.write_put_edge(context, "U11456af7d414", "Bad1c69de7837", -2.0);
  graph.write_put_edge(context, "U998403e8a30d", "U000000000000",  1.0);
  graph.write_put_edge(context, "U95f3426b8e5d", "C992d8370db6b",  1.0);
  graph.write_put_edge(context, "U6a774cf456f7", "U000000000000",  1.0);
  graph.write_put_edge(context, "U80e22da6d8c4", "B45d72e29f004",  3.0);
  graph.write_put_edge(context, "U8a78048d60f7", "B3b3f2ecde430", -1.0);
  graph.write_put_edge(context, "U1bcba4fd7175", "Bc4addf09b79f",  3.0);
}

#[test]
fn encoding_serde() {
  let in_command : String = "foo".into();
  let in_context : &str = "bar";
  let in_arg1    : &str = "baz";
  let in_arg2    : &str = "bus";

  let payload = rmp_serde::to_vec(&(
    in_command.clone(),
    in_context,
    rmp_serde::to_vec(&(in_arg1, in_arg2)).unwrap()
  )).unwrap();

  let out_command : &str;
  let out_context : String;
  let _out_args   : Vec<u8>;

  (out_command, out_context, _out_args) = rmp_serde::from_slice(payload.as_slice()).unwrap();

  assert_eq!(out_command, in_command);
  assert_eq!(out_context, in_context);
}

#[test]
fn encoding_response() {
  let foo     = ("foo".to_string(), 1, 2, 3);
  let payload = encode_response(&foo).unwrap();

  let bar : (String, i32, i32, i32) = decode_response(&payload).unwrap();

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

  put_testing_edges(&mut graph, "");

  graph.write_recalculate_zero();

  let res : Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "U000000000000", false, 0, 10000);

  let n = res.len();

  println!("Got {} edges", n);

  assert!(n > 25);
  assert!(n < 120);
}

#[test]
fn graph_sort_order() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph, "");

  graph.write_recalculate_zero();

  let res : Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "U000000000000", false, 0, 10000);

  for n in 1..res.len() {
    assert!(res[n - 1].2.abs() >= res[n].2.abs());
  }
}

#[test]
fn recalculate_zero_graph_duplicates() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph, "");

  graph.write_recalculate_zero();

  let res : Vec<_> =
    graph.read_graph("", "U000000000000", "Ub01f4ad1b03f", false, 0, 10000);

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

  put_testing_edges(&mut graph, "");

  graph.write_recalculate_zero();

  let res : Vec<_> =
    graph.read_graph("", "Uadeb43da4abb", "U000000000000", true, 0, 10000);

  let n = res.len();

  println!("Got {} edges", n);
  assert!(n > 25);
  assert!(n < 120);
}

#[test]
fn recalculate_zero_graph_focus_beacon() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph, "");

  graph.write_recalculate_zero();

  let res : Vec<_> =
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

  put_testing_edges(&mut graph, "");
  graph.write_recalculate_zero();
  graph.reset();
  put_testing_edges(&mut graph, "");
  graph.write_create_context("X");
  graph.write_create_context("Y");
  graph.write_create_context("Z");
  graph.write_recalculate_zero();

  let begin    = SystemTime::now();
  let get_time = || SystemTime::now().duration_since(begin).unwrap().as_millis();

  graph.read_graph("", "Uadeb43da4abb", "U000000000000", true, 0, 10000);

  assert!(get_time() < 200);
}

#[test]
fn recalculate_zero_scores() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph, "");

  graph.write_recalculate_zero();

  let res : Vec<_> =
    graph.read_scores("", "Uadeb43da4abb", "B", true, 100.0, false, -100.0, false, 0, u32::MAX);

  let n = res.len();

  println!("Got {} edges", n);
  assert!(n > 5);
  assert!(n < 80);
}

#[test]
fn scores_sort_order() {
  let mut graph = AugMultiGraph::new();

  put_testing_edges(&mut graph, "");

  graph.write_recalculate_zero();

  let res : Vec<_> =
    graph.read_scores("", "Uadeb43da4abb", "B", true, 100.0, false, -100.0, false, 0, u32::MAX);

  for n in 1..res.len() {
    assert!(res[n - 1].2.abs() >= res[n].2.abs());
  }
}

#[test]
fn edge_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 1.5);

  let edges : Vec<_> = graph.read_edges("");

  let edges_expected : Vec<(String, String, Weight)> = vec![
    ("U1".to_string(), "U2".to_string(), 1.5)
  ];

  assert_eq!(edges, edges_expected);
}

#[test]
fn edge_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 1.5);

  let edges : Vec<_> = graph.read_edges("X");

  let edges_expected : Vec<(String, String, Weight)> = vec![
    ("U1".to_string(), "U2".to_string(), 1.5)
  ];

  assert_eq!(edges, edges_expected);
}

#[test]
fn null_context_is_sum() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "B1", "U2", 1.0);
  graph.write_put_edge("Y", "B1", "U2", 2.0);

  let edges : Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected : Vec<(String, String, Weight)> = vec![
    ("B1".to_string(), "U2".to_string(), 3.0)
  ];

  assert_eq!(edges, edges_expected);
}

#[test]
fn null_context_contains_all_users() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 1.0);
  graph.write_put_edge("Y", "U1", "U3", 2.0);

  let edges : Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected : Vec<(String, String, Weight)> = vec![
    ("U1".to_string(), "U2".to_string(), 1.0),
    ("U1".to_string(), "U3".to_string(), 2.0),
  ];

  assert_eq!(edges, edges_expected);
}


#[test]
fn user_edges_dup() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 1.0);
  graph.write_put_edge("X", "U1", "U3", 2.0);
  graph.write_create_context("Y");

  let edges : Vec<(String, String, Weight)> = graph.read_edges("Y");

  let edges_expected : Vec<(String, String, Weight)> = vec![
    ("U1".to_string(), "U2".to_string(), 1.0),
    ("U1".to_string(), "U3".to_string(), 2.0),
  ];

  assert_eq!(edges, edges_expected);
}

#[test]
fn non_user_edges_no_dup() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "C2", 1.0);
  graph.write_put_edge("X", "U1", "C3", 2.0);
  graph.write_create_context("Y");

  let edges : Vec<(String, String, Weight)> = graph.read_edges("Y");

  assert_eq!(edges.len(), 0);
}

#[test]
fn delete_contexted_edge() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "B1", "U2", 1.0);
  graph.write_put_edge("Y", "B1", "U2", 2.0);
  graph.write_delete_edge("X", "B1", "U2");

  let edges : Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected : Vec<(String, String, Weight)> = vec![
    ("B1".to_string(), "U2".to_string(), 2.0)
  ];

  assert_eq!(edges, edges_expected);
}

#[test]
fn null_context_invariant() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "B1", "B2", 1.0);
  graph.write_put_edge("Y", "B1", "B2", 2.0);
  graph.write_delete_edge("X", "B1", "B2");
  graph.write_put_edge("X", "B1", "B2", 1.0);

  let edges : Vec<(String, String, Weight)> = graph.read_edges("");

  let edges_expected : Vec<(String, String, Weight)> = vec![
    ("B1".to_string(), "B2".to_string(), 3.0)
  ];

  assert_eq!(edges, edges_expected);
}

#[test]
fn scores_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0);
  graph.write_put_edge("", "U1", "U3", 1.0);
  graph.write_put_edge("", "U2", "U3", 3.0);

  let res : Vec<_> = graph.read_scores("", "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

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
fn scores_reversed() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0);
  graph.write_put_edge("", "U1", "U3", 1.0);
  graph.write_put_edge("", "U2", "U3", 3.0);
  graph.write_put_edge("", "U2", "U1", 4.0);
  graph.write_put_edge("", "U3", "U1", -5.0);

  let res : Vec<_> = graph.read_scores("", "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

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
        assert!(x.3 <  0.1);
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

  graph.write_put_edge("X", "U1", "U2", 2.0);
  graph.write_put_edge("X", "U1", "U3", 1.0);
  graph.write_put_edge("X", "U2", "U3", 3.0);

  let res : Vec<_> = graph.read_scores("X", "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

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

  graph.write_put_edge("X", "B1", "B2", 2.0);
  graph.write_put_edge("X", "B1", "B3", 1.0);
  graph.write_put_edge("X", "B2", "B3", 3.0);

  let res : Vec<_> = graph.read_scores("Y", "B1", "B", false, 10.0, false, 0.0, false, 0, u32::MAX);

  assert_eq!(res.len(), 0);
}

#[test]
fn scores_reset_smoke() {
  let mut graph_read  = AugMultiGraph::new();
  let mut graph_write = AugMultiGraph::new();

  graph_write.write_put_edge("X", "U1", "U2", 2.0);
  graph_write.write_put_edge("X", "U1", "U3", 1.0);
  graph_write.write_put_edge("X", "U2", "U3", 3.0);

  graph_read.copy_from(&graph_write);
  let res : Vec<_> = graph_read.read_scores("X", "U1", "U", false, 10.0, false, 0.0, false, 0, 2147483647);

  assert_eq!(res.len(), 3);

  graph_write.reset();

  graph_write.write_put_edge("X", "U1", "U2", 2.0);
  graph_write.write_put_edge("X", "U1", "U3", 1.0);
  graph_write.write_put_edge("X", "U2", "U3", 3.0);

  graph_read.copy_from(&graph_write);
  let res : Vec<_> = graph_read.read_scores("X", "U1", "U", false, 2147483647.0, false, -2147483648.0, false, 0, 2147483647);

  assert_eq!(res.len(), 3);
}

#[test]
fn scores_self() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "B1", "B2", 2.0);
  graph.write_put_edge("X", "B1", "B3", 1.0);
  graph.write_put_edge("X", "B2", "U1", 3.0);
  graph.write_create_context("Y");

  let res : Vec<_> = graph.read_scores("Y", "U1", "U", false, 10.0, false, 0.0, false, 0, u32::MAX);

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U1");
  assert!(res[0].2 > 0.999);
  assert!(res[0].2 < 1.001);
}

#[test]
fn node_list_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0);
  graph.write_put_edge("", "U1", "U3", 1.0);
  graph.write_put_edge("", "U3", "U2", 3.0);

  let res : Vec<(String,)> = graph.read_node_list();

  let mut has_u1 = false;
  let mut has_u2 = false;
  let mut has_u3 = false;

  for (x,) in res {
    match x.as_str() {
      "U1" => has_u1 = true,
      "U2" => has_u2 = true,
      "U3" => has_u3 = true,
      _    => assert!(false),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
  assert!(has_u3);
}

#[test]
fn node_list_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 2.0);
  graph.write_put_edge("X", "U1", "U3", 1.0);
  graph.write_put_edge("X", "U3", "U2", 3.0);

  let res : Vec<(String,)> = graph.read_node_list();

  let mut has_u1 = false;
  let mut has_u2 = false;
  let mut has_u3 = false;

  for (x,) in res {
    match x.as_str() {
      "U1" => has_u1 = true,
      "U2" => has_u2 = true,
      "U3" => has_u3 = true,
      _    => assert!(false),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
  assert!(has_u3);
}

#[test]
fn node_list_mixed() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0);
  graph.write_put_edge("X", "U1", "U3", 1.0);
  graph.write_put_edge("Y", "U3", "U2", 3.0);

  let res : Vec<(String,)> = graph.read_node_list();

  let mut has_u1 = false;
  let mut has_u2 = false;
  let mut has_u3 = false;

  for (x,) in res {
    match x.as_str() {
      "U1" => has_u1 = true,
      "U2" => has_u2 = true,
      "U3" => has_u3 = true,
      _    => assert!(false),
    }
  }

  assert!(has_u1);
  assert!(has_u2);
  assert!(has_u3);
}

#[test]
fn node_score_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0);
  graph.write_put_edge("", "U1", "U3", 1.0);
  graph.write_put_edge("", "U3", "U2", 3.0);

  let res : Vec<_> = graph.read_node_score("", "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U2");
  assert!(res[0].2 > 0.3);
  assert!(res[0].2 < 0.45);
}

#[test]
fn node_score_reversed() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 2.0);
  graph.write_put_edge("", "U1", "U3", 1.0);
  graph.write_put_edge("", "U3", "U2", 3.0);
  graph.write_put_edge("", "U2", "U1", 4.0);

  let res : Vec<_> = graph.read_node_score("", "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U2");
  assert!(res[0].2 > 0.3);
  assert!(res[0].2 < 0.45);
  assert!(res[0].3 > 0.3);
  assert!(res[0].3 < 0.45);
}

#[test]
fn node_score_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 2.0);
  graph.write_put_edge("X", "U1", "U3", 1.0);
  graph.write_put_edge("X", "U3", "U2", 3.0);

  let res : Vec<_> = graph.read_node_score("X", "U1", "U2");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U2");
  assert!(res[0].2 > 0.3);
  assert!(res[0].2 < 0.45);
}

#[test]
fn mutual_scores_uncontexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 3.0);
  graph.write_put_edge("", "U1", "U3", 1.0);
  graph.write_put_edge("", "U2", "U1", 2.0);
  graph.write_put_edge("", "U2", "U3", 4.0);
  graph.write_put_edge("", "U3", "U1", 3.0);
  graph.write_put_edge("", "U3", "U2", 2.0);

  let res : Vec<_> = graph.read_mutual_scores("", "U1");

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
        assert!(x.3 > 0.25);
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
fn mutual_scores_self() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 3.0);
  graph.write_delete_edge("", "U1", "U2");

  let res : Vec<_> = graph.read_mutual_scores("", "U1");

  assert_eq!(res.len(), 1);
  assert_eq!(res[0].0, "U1");
  assert_eq!(res[0].1, "U1");
  assert!(res[0].2 > 0.999);
  assert!(res[0].2 < 1.001);
  assert!(res[0].3 > 0.999);
  assert!(res[0].3 < 1.001);
}

#[test]
fn mutual_scores_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 3.0);
  graph.write_put_edge("X", "U1", "U3", 1.0);
  graph.write_put_edge("X", "U2", "U1", 2.0);
  graph.write_put_edge("X", "U2", "U3", 4.0);
  graph.write_put_edge("X", "U3", "U1", 3.0);
  graph.write_put_edge("X", "U3", "U2", 2.0);

  let res : Vec<_> = graph.read_mutual_scores("X", "U1");

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

  graph.write_put_edge("", "U1", "U2", 2.0);
  graph.write_put_edge("", "U1", "U3", 1.0);
  graph.write_put_edge("", "U2", "U3", 3.0);

  let res : Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

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

  graph.write_put_edge("", "U1", "U2", 2.0);
  graph.write_put_edge("", "U1", "U3", 1.0);
  graph.write_put_edge("", "U2", "U3", 3.0);
  graph.write_put_edge("", "U2", "U1", 4.0);

  let res : Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

  assert_eq!(res.len(), 3);

  for x in res {
    match x.0.as_str() {
      "U1" => {
        assert_eq!(x.1, "U2");
        assert!(x.2 > 0.6);
        assert!(x.2 < 0.7);
        assert!(x.3 > 0.15);
        assert!(x.3 < 0.3);
      },

      "U2" => {
        if x.1 == "U1" {
          assert!(x.2 > 0.5);
          assert!(x.2 < 0.6);
          assert!(x.3 > 0.4);
        }

        if x.1 == "U3" {
          assert!(x.2 > 0.39);
          assert!(x.2 < 0.49);
          assert!(x.3 > -0.1);
          assert!(x.3 < 0.1);
        }
      },

      _ => panic!(),
    }
  }
}

#[test]
fn graph_contexted() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "U2", 2.0);
  graph.write_put_edge("X", "U1", "U3", 1.0);
  graph.write_put_edge("X", "U2", "U3", 3.0);

  let res : Vec<_> = graph.read_graph("X", "U1", "U2", false, 0, 10000);

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

  graph.write_put_edge("", "U1", "U2", 2.0);
  graph.write_put_edge("", "U1", "U3", 1.0);
  graph.write_put_edge("", "U2", "U3", 3.0);

  graph.write_delete_edge("", "U1", "U2");
  graph.write_delete_edge("", "U1", "U3");
  graph.write_delete_edge("", "U2", "U3");

  let res : Vec<_> = graph.read_graph("", "U1", "U2", false, 0, 10000);

  for x in res.iter() {
    println!("{} -> {}: {}", x.0, x.1, x.2);
  }

  assert_eq!(res.len(), 0);
}

#[test]
fn graph_removed_edge() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "B2", 1.0);
  graph.write_put_edge("", "B2", "U1", 2.0);
  graph.write_put_edge("", "B2", "C2", 1.0);
  graph.write_put_edge("", "B2", "C3", 1.5);
  graph.write_put_edge("", "B2", "C4", 3.0);

  graph.write_delete_edge("", "U1", "B2");

  let res : Vec<_> = graph.read_graph("", "U1", "B2", false, 0, 10000);

  for x in res.iter() {
    println!("{} -> {}: {}", x.0, x.1, x.2);
  }

  assert_eq!(res.len(), 0);
}

#[test]
fn new_edges_fetch() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 1.0);

  assert_eq!(graph.write_fetch_new_edges("U1", "B").len(), 0);

  graph.write_put_edge("", "U1", "B3", 2.0);
  graph.write_put_edge("", "U2", "B4", 3.0);

  let beacons = graph.write_fetch_new_edges("U1", "B");

  assert_eq!(beacons.len(), 2);
  assert_eq!(beacons[0].0, "B3");
  assert_eq!(beacons[1].0, "B4");

  assert_eq!(graph.write_fetch_new_edges("U1", "B").len(), 0);
}

#[test]
fn new_edges_filter() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("", "U1", "U2", 1.0);

  assert_eq!(graph.write_fetch_new_edges("U1", "B").len(), 0);

  graph.write_put_edge("", "U1", "B3", 2.0);
  graph.write_put_edge("", "U2", "B4", 3.0);

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

  graph.write_put_edge("X", "U1", "U2", 1.0);
  graph.write_put_edge("X", "U1", "C2", 2.0);
  graph.write_create_context("Y");

  let edges : Vec<(String, String, Weight)> = graph.read_edges("Y");

  assert_eq!(edges.len(), 1);
  assert_eq!(edges[0].0, "U1");
  assert_eq!(edges[0].1, "U2");
  assert!(edges[0].2 > 0.999);
  assert!(edges[0].2 < 1.001);
}

#[test]
fn context_already_exist() {
  let mut graph = AugMultiGraph::new();

  graph.write_put_edge("X", "U1", "C2", 1.0);
  graph.write_create_context("X");

  let edges : Vec<(String, String, Weight)> = graph.read_edges("X");

  assert_eq!(edges.len(), 1);
  assert_eq!(edges[0].0, "U1");
  assert_eq!(edges[0].1, "C2");
  assert!(edges[0].2 > 0.999);
  assert!(edges[0].2 < 1.001);
}
