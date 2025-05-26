mod utils;

#[cfg(test)]
mod tests {
  use meritrank_core::{Graph, MeritRank, NodeId};
  static IDS: &[u32] = &[
    253, 299, 407, 433, 312, 477, 45, 508, 102, 287, 124, 356, 507, 523, 332,
    355, 367, 370, 527, 245, 238, 413, 108, 122, 518, 443, 439, 429, 321, 97,
    162, 488, 44, 465, 368, 446, 365, 239, 371, 485, 334, 199, 525, 179, 18,
    404, 6, 0, 416, 495, 160, 351, 169, 214, 295, 291, 30, 55, 530, 164, 535,
    94, 516, 419, 366, 235, 273, 188, 192, 167, 360, 412, 381, 352, 482, 318,
    210, 424, 379, 358, 4, 101, 31, 496, 457, 16, 313, 382, 490, 435, 484, 375,
    166, 144, 78, 289, 533, 190, 136, 276, 293, 519, 43, 201, 406, 521, 13,
    515, 491, 74, 41, 335, 230, 56, 8, 265, 336, 42, 93, 300, 178, 84, 35, 40,
    143, 514, 127, 130, 466, 493, 183, 50, 58, 263, 476, 79, 217, 177, 315,
    248, 502, 267, 510, 228, 442, 65, 70, 203, 319, 62, 378, 423, 444, 471,
    480, 151, 282, 489, 21, 141, 114, 59, 306, 325, 219, 280, 57, 221, 506,
    410, 67, 91, 113, 425, 198, 272, 353, 451, 494, 456, 513, 53, 526, 420, 80,
    2, 498, 499, 509, 186, 234, 430, 224, 222, 28, 415, 447, 257, 427, 487,
    180, 133, 417, 532, 524, 483, 297, 68, 226, 463, 99, 403, 441, 77, 85, 348,
    504, 12, 388, 342, 362, 534, 333, 200, 384, 33, 440, 475, 460, 517, 255,
    537, 275, 145, 87, 434, 39, 262, 285, 469, 390, 343, 36, 52, 90, 15, 421,
    196, 163, 400, 505, 422, 330, 464, 11, 47, 314, 76, 344, 520, 459, 218,
    536, 492, 149, 120, 323, 346, 3, 104, 19, 63, 107, 481, 138, 23, 153, 207,
    411, 193, 232, 327, 512, 159, 105, 48, 270, 9, 393, 529, 350, 472, 181,
    380, 414, 197, 189, 431, 110, 284, 426, 142, 259, 303, 402, 211, 283, 288,
    392, 281, 478, 215, 385, 528, 82, 474, 117, 418, 448, 150, 132, 397, 467,
    473, 479, 27, 29, 386, 195, 497, 500, 69, 531, 454, 349, 522,
  ];

  use crate::utils::read_edges_from_csv;
  use std::time::SystemTime;

  pub fn load_graph_from_csv(filename: &str) -> MeritRank {
    let mut rank = MeritRank::new(Graph::new());
    let mut max_node_id = 0;
    for edge in read_edges_from_csv("tests/graph_with_zero.csv") {
      while edge.src >= max_node_id || edge.dst >= max_node_id {
        max_node_id += 1;
        rank.get_new_nodeid();
      }
      rank.set_edge(edge.src as NodeId, edge.dst as NodeId, edge.weight);
    }
    rank
  }

  #[test]
  fn smoke_perf_with_zero() {
    let numwalks = 100;
    let mut rank = load_graph_from_csv("tests/graph_with_zero.csv");
    for ego in IDS {
      rank.calculate(*ego as NodeId, numwalks).unwrap();
    }
    let begin = SystemTime::now();
    let get_time =
      || SystemTime::now().duration_since(begin).unwrap().as_millis();

    //  Should be fast!

    println!("1");
    rank.set_edge(3, 35, 0.6);
    assert!(get_time() < 80000);

    println!("2");
    rank.set_edge(3, 27, 0.6);
    assert!(get_time() < 80000);
  }
  #[cfg(feature = "expensive_tests")]
  #[test]
  fn smoke_memory() {
    let numwalks = 10000;
    let mut rank = load_graph_from_csv("tests/graph_with_zero.csv");
    for ego in IDS {
      rank.calculate(*ego as NodeId, numwalks).unwrap();
    }
  }
}
