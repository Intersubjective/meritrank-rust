//  ================================================================
//
//    astar.rs
//
//  Iterative general implementation for
//  the A* graph search algorithm
//
//  ----------------------------------------------------------------
//
//    (C) 2024 Mitya Selivanov <guattari.tech>, MIT License
//
//  ================================================================

#[allow(non_camel_case_types)]
mod astar_internal
{
  use std::{fmt::Debug, ops::Add};

  #[derive(Debug, Clone, PartialEq, Default)]
  pub struct Neighbor_Request<Node_Id>
  {
    pub node:  Node_Id,
    pub index: usize,
  }

  #[derive(Debug, Clone, PartialEq)]
  pub enum Status<Node_Id>
  {
    PROGRESS,
    SUCCESS,
    FAIL,
    OUT_OF_MEMORY,
    NEIGHBOR(Neighbor_Request<Node_Id>),
  }

  #[derive(Debug, Clone, Default)]
  pub struct Link<Node_Id, Cost>
  {
    pub neighbor:       Node_Id,
    pub exact_distance: Cost,
    pub estimate:       Cost,
  }

  #[derive(Debug, Clone, Default)]
  pub struct Node<Node_Id, Cost>
  {
    pub id:             Node_Id,
    pub previous:       Option<Node_Id>,
    pub exact_distance: Cost,
    pub estimate:       Cost,
    pub count:          usize,
  }

  #[derive(Debug, Clone, PartialEq)]
  pub enum Stage
  {
    SEARCH_NEAREST,
    ENUM_NEIGHBORS,
  }

  #[derive(Clone)]
  pub struct State<Node_Id, Cost>
  {
    pub stage:            Stage,
    pub num_open:         usize,
    pub num_closed:       usize,
    pub source:           Node_Id,
    pub destination:      Node_Id,
    pub closest_index:    usize,
    pub closest_estimate: Cost,
    pub node:             Option<Node<Node_Id, Cost>>,
    pub neighbor_index:   usize,
  }

  pub fn init<Node_Id, Cost>(
    open: &mut [Node<Node_Id, Cost>],
    source: Node_Id,
    destination: Node_Id,
    max_cost: Cost,
  ) -> State<Node_Id, Cost>
  where
    Node_Id: Clone,
    Cost: Clone + Default,
  {
    if open.len() != 0 {
      open[0] = Node::<Node_Id, Cost> {
        id:             source.clone(),
        previous:       None,
        exact_distance: Cost::default(),
        estimate:       max_cost.clone(),
        count:          1,
      };
    }

    return State {
      stage: Stage::SEARCH_NEAREST,
      num_open: if open.len() != 0 { 1 } else { 0 },
      num_closed: 0,
      source,
      destination,
      closest_index: 0,
      closest_estimate: max_cost,
      node: None,
      neighbor_index: 0,
    };
  }

  pub fn path<Node_Id, Cost>(
    closed: &[Node<Node_Id, Cost>],
    state: &State<Node_Id, Cost>,
    node_ids: &mut [Node_Id],
  ) -> usize
  where
    Node_Id: Clone + PartialEq,
  {
    if closed.is_empty()
      || state.closest_index >= state.num_closed
      || node_ids.len() < state.num_closed
    {
      return 0;
    }

    let mut current = state.closest_index;

    let mut num_nodes: usize = 1;
    node_ids[0] = closed[current].id.clone();

    loop {
      if node_ids[num_nodes - 1] == state.source {
        break;
      }

      if num_nodes >= state.num_closed {
        return 0;
      }

      let mut index = usize::MAX;
      for i in 0..state.num_closed {
        if Some(closed[i].id.clone()) == closed[current].previous {
          index = i;
          break;
        }
      }

      if index == usize::MAX {
        return 0;
      }

      node_ids[num_nodes] = closed[index].id.clone();
      num_nodes += 1;
      current = index;
    }

    for i in 0..num_nodes / 2 {
      let x = node_ids[i].clone();
      node_ids[i] = node_ids[num_nodes - 1 - i].clone();
      node_ids[num_nodes - 1 - i] = x;
    }

    return num_nodes;
  }

  pub fn iteration<Node_Id, Cost>(
    open: &mut [Node<Node_Id, Cost>],
    closed: &mut [Node<Node_Id, Cost>],
    state: &mut State<Node_Id, Cost>,
    neighbor: Option<Link<Node_Id, Cost>>,
  ) -> Status<Node_Id>
  where
    Node_Id: Debug + Clone + Default + PartialEq,
    Cost: Debug + Clone + Default + PartialOrd + Add<Output = Cost>,
  {
    match state.stage {
      Stage::SEARCH_NEAREST => {
        if state.num_open == 0 {
          return Status::FAIL;
        }

        //  Check if we need more memory
        //
        if state.num_open + 1 >= open.len() || state.num_closed + 2 >= closed.len() {
          return Status::OUT_OF_MEMORY;
        }

        //  Find the nearest node to the destination in the open set
        //

        let mut index_in_open: usize = 0;
        for index in 1..state.num_open {
          let node = open[index].clone();
          let nearest = open[index_in_open].clone();
          if node.exact_distance + node.estimate < nearest.exact_distance + nearest.estimate {
            index_in_open = index;
          }
        }

        let nearest_node = open[index_in_open].clone();
        state.node = Some(nearest_node.clone());
        if index_in_open != state.num_open - 1 {
          open[index_in_open] = open[state.num_open - 1].clone();
        }
        state.num_open -= 1;

        //  Check if we reached the destination
        //
        if nearest_node.id == state.destination {
          state.closest_index = state.num_closed;
          state.closest_estimate = Cost::default();
          closed[state.num_closed] = nearest_node;
          state.num_closed += 1;

          //  Finish the search
          return Status::SUCCESS;
        }

        //  Proceed to the neighbors enumeration stage
        //

        state.stage = Stage::ENUM_NEIGHBORS;
        state.neighbor_index = 0;

        return Status::NEIGHBOR(Neighbor_Request {
          node:  nearest_node.id,
          index: state.neighbor_index,
        });
      },

      Stage::ENUM_NEIGHBORS => {
        match state.node.clone() {
          None => panic!(),
          Some(nearest_node) => {
            match neighbor {
              Some(link) => {
                //  Check if we need more memory
                //
                if state.num_open + 1 >= open.len() {
                  return Status::OUT_OF_MEMORY;
                }

                //  Calculate distance estimations
                //

                let exact_distance =
                  nearest_node.clone().exact_distance + link.clone().exact_distance;
                let estimate = link.clone().estimate;

                let neighbor_node = Node {
                  id: link.neighbor,
                  previous: Some(nearest_node.clone().id),
                  exact_distance,
                  estimate,
                  count: nearest_node.count + 1,
                };

                //  Check if we reached the destination
                //
                if neighbor_node.id == state.destination {
                  state.closest_index = state.num_closed + 1;
                  state.closest_estimate = Cost::default();
                  closed[state.num_closed] = nearest_node;
                  closed[state.num_closed + 1] = neighbor_node.clone();
                  state.node = None;
                  state.num_closed += 2;

                  //  Finish the search
                  return Status::SUCCESS;
                }

                //  Check if this node is already in the closed set
                //

                let mut index_in_closed = usize::MAX;
                for i in 0..state.num_closed {
                  if closed[i].id == neighbor_node.id {
                    index_in_closed = i;
                    break;
                  }
                }

                if index_in_closed != usize::MAX {
                  //  Check if this node has a better distance
                  if neighbor_node.exact_distance < closed[index_in_closed].exact_distance {
                    if neighbor_node.estimate < state.closest_estimate {
                      state.closest_index = index_in_closed;
                      state.closest_estimate = neighbor_node.clone().estimate;
                    }

                    //  Replace the node
                    closed[index_in_closed] = neighbor_node;
                  }

                  //  Skip this node and proceed to the next neighbor node
                  //

                  state.neighbor_index += 1;

                  return Status::NEIGHBOR(Neighbor_Request {
                    node:  nearest_node.id,
                    index: state.neighbor_index,
                  });
                }

                //  Check if this node is already in the open set
                //

                let mut index_in_open: usize = usize::MAX;
                for i in 0..state.num_open {
                  if open[i].id == neighbor_node.id {
                    index_in_open = i;
                    break;
                  }
                }

                if index_in_open != usize::MAX {
                  //  Check if this node has a better distance
                  if neighbor_node.exact_distance < open[index_in_open].exact_distance {
                    //  Replace the node
                    open[index_in_open] = neighbor_node;
                  }

                  //  Skip this node and proceed to the next neighbor node
                  //

                  state.neighbor_index += 1;

                  return Status::NEIGHBOR(Neighbor_Request {
                    node:  nearest_node.id,
                    index: state.neighbor_index,
                  });
                }

                open[state.num_open] = neighbor_node;
                state.num_open += 1;

                //  Proceed to the next neighbor node
                //

                state.neighbor_index += 1;

                return Status::NEIGHBOR(Neighbor_Request {
                  node:  nearest_node.id,
                  index: state.neighbor_index,
                });
              },

              None => {
                if nearest_node.estimate < state.closest_estimate {
                  state.closest_index = state.num_closed;
                  state.closest_estimate = nearest_node.clone().estimate;
                }

                closed[state.num_closed] = nearest_node;
                state.node = None;
                state.num_closed += 1;

                //  Proceed to the nearest node search
                //

                state.stage = Stage::SEARCH_NEAREST;

                return Status::PROGRESS;
              },
            }
          },
        }
      },
    };
  }
}

pub use astar_internal::*;

//  ================================================================
//
//    Testing
//
//  ================================================================

#[cfg(test)]
mod tests
{
  use super::*;

  #[test]
  fn path_exists()
  {
    let graph: Vec<((i64, i64), i64)> = vec![
      ((0, 1), 5),
      ((0, 2), 3),
      ((1, 3), 4),
      ((2, 4), 1),
      ((3, 5), 10),
      ((4, 6), 1),
      ((6, 7), 1),
      ((7, 5), 1),
    ];

    let get_neighbor = |id: i64, index: usize| -> Option<Link<i64, i64>> {
      let mut k: usize = 0;
      for ((src, dst), cost) in graph.clone() {
        if src == id {
          if k == index {
            return Some(Link::<i64, i64> {
              neighbor:       dst,
              exact_distance: cost,
              estimate:       (8 - dst).abs(),
            });
          } else {
            k += 1;
          }
        }
      }
      return None;
    };

    let mut open: Vec<Node<i64, i64>> = vec![];
    let mut closed: Vec<Node<i64, i64>> = vec![];

    open.resize(1024, Node::default());
    closed.resize(1024, Node::default());

    let mut state = init(&mut open, 0i64, 5i64, i64::MAX);

    let mut steps = 0;
    let mut neighbor = None;
    loop {
      steps += 1;

      match iteration(&mut open, &mut closed, &mut state, neighbor.clone()) {
        Status::NEIGHBOR(request) => neighbor = get_neighbor(request.node, request.index),
        Status::SUCCESS => break,
        Status::PROGRESS => {},
        _ => assert!(false),
      };
    }

    let mut v: Vec<i64> = vec![];
    v.resize(state.num_closed, 0);
    let n = path(&closed, &state, &mut v);
    v.resize(n, 0);

    assert_eq!(steps, 15);
    assert_eq!(v.len(), 6);
    assert_eq!(v[0], 0);
    assert_eq!(v[1], 2);
    assert_eq!(v[2], 4);
    assert_eq!(v[3], 6);
    assert_eq!(v[4], 7);
    assert_eq!(v[5], 5);
  }

  #[test]
  fn out_of_memory()
  {
    let graph: Vec<((i64, i64), i64)> = vec![
      ((0, 1), 5),
      ((0, 2), 3),
      ((1, 3), 4),
      ((2, 4), 1),
      ((3, 5), 10),
      ((4, 6), 1),
      ((6, 7), 1),
      ((7, 5), 1),
    ];

    let get_neighbor = |id: i64, index: usize| -> Option<Link<i64, i64>> {
      let mut k: usize = 0;
      for ((src, dst), cost) in graph.clone() {
        if src == id {
          if k == index {
            return Some(Link::<i64, i64> {
              neighbor:       dst,
              exact_distance: cost,
              estimate:       (8 - dst).abs(),
            });
          } else {
            k += 1;
          }
        }
      }
      return None;
    };

    let mut open: Vec<Node<i64, i64>> = vec![Node::default()];
    let mut closed: Vec<Node<i64, i64>> = vec![];

    let mut state = init(&mut open, 0i64, 5i64, i64::MAX);

    let mut steps = 0;
    let mut neighbor = None;
    loop {
      steps += 1;

      match iteration(&mut open, &mut closed, &mut state, neighbor.clone()) {
        Status::NEIGHBOR(request) => neighbor = get_neighbor(request.node, request.index),
        Status::SUCCESS => break,
        Status::PROGRESS => {},
        Status::OUT_OF_MEMORY => {
          open.resize(open.len() + 1024, Node::default());
          closed.resize(closed.len() + 1024, Node::default());
        },
        _ => assert!(false),
      };
    }

    let mut v: Vec<i64> = vec![];
    v.resize(state.num_closed, 0);
    let n = path(&closed, &state, &mut v);
    v.resize(n, 0);

    assert_eq!(steps, 16);
    assert_eq!(v.len(), 6);
    assert_eq!(v[0], 0);
    assert_eq!(v[1], 2);
    assert_eq!(v[2], 4);
    assert_eq!(v[3], 6);
    assert_eq!(v[4], 7);
    assert_eq!(v[5], 5);
  }

  #[test]
  fn path_does_not_exist()
  {
    let graph: Vec<((i64, i64), i64)> = vec![
      ((0, 1), 5),
      ((0, 2), 3),
      ((1, 3), 4),
      ((2, 4), 1),
      ((3, 5), 1),
      ((4, 6), 10),
      ((6, 7), 1),
      ((7, 5), 1),
    ];

    let get_neighbor = |id: i64, index: usize| -> Option<Link<i64, i64>> {
      let mut k: usize = 0;
      for ((src, dst), cost) in graph.clone() {
        if src == id {
          if k == index {
            return Some(Link::<i64, i64> {
              neighbor:       dst,
              exact_distance: cost,
              estimate:       (15 - dst).abs(),
            });
          } else {
            k += 1;
          }
        }
      }
      return None;
    };

    let mut open: Vec<Node<i64, i64>> = vec![];
    let mut closed: Vec<Node<i64, i64>> = vec![];

    open.resize(1024, Node::default());
    closed.resize(1024, Node::default());

    let mut state = init(&mut open, 0i64, 15i64, i64::MAX);

    let mut steps = 0;
    let mut neighbor = None;
    loop {
      steps += 1;

      match iteration(&mut open, &mut closed, &mut state, neighbor.clone()) {
        Status::NEIGHBOR(request) => neighbor = get_neighbor(request.node, request.index),
        Status::FAIL => break,
        Status::PROGRESS => {},
        _ => assert!(false),
      };
    }

    let mut v: Vec<i64> = vec![];
    v.resize(state.num_closed, 0);
    let n = path(&closed, &state, &mut v);
    v.resize(n, 0);

    assert_eq!(steps, 25);
    assert_eq!(v.len(), 5);
    assert_eq!(v[0], 0);
    assert_eq!(v[1], 2);
    assert_eq!(v[2], 4);
    assert_eq!(v[3], 6);
    assert_eq!(v[4], 7);
  }

  #[test]
  fn empty_path()
  {
    let graph: Vec<((i64, i64), i64)> = vec![
      ((0, 1), 5),
      ((0, 2), 3),
      ((1, 3), 4),
      ((2, 4), 1),
      ((3, 5), 1),
      ((4, 6), 10),
      ((6, 7), 1),
      ((7, 5), 1),
    ];

    let get_neighbor = |id: i64, index: usize| -> Option<Link<i64, i64>> {
      let mut k: usize = 0;
      for ((src, dst), cost) in graph.clone() {
        if src == id {
          if k == index {
            return Some(Link::<i64, i64> {
              neighbor:       dst,
              exact_distance: cost,
              estimate:       (2 - dst).abs(),
            });
          } else {
            k += 1;
          }
        }
      }
      return None;
    };

    let mut open: Vec<Node<i64, i64>> = vec![];
    let mut closed: Vec<Node<i64, i64>> = vec![];

    open.resize(1024, Node::default());
    closed.resize(1024, Node::default());

    let mut state = init(&mut open, 2i64, 2i64, i64::MAX);

    let mut steps = 0;
    let mut neighbor = None;
    loop {
      steps += 1;

      match iteration(&mut open, &mut closed, &mut state, neighbor.clone()) {
        Status::NEIGHBOR(request) => neighbor = get_neighbor(request.node, request.index),
        Status::SUCCESS => break,
        Status::PROGRESS => {},
        _ => assert!(false),
      };
    }

    let mut v: Vec<i64> = vec![];
    v.resize(state.num_closed, 0);
    let n = path(&closed, &state, &mut v);
    v.resize(n, 0);

    assert_eq!(steps, 1);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0], 2);
  }

  #[test]
  fn cyclic()
  {
    let graph: Vec<((i64, i64), i64)> = vec![
      ((0, 1), 10),
      ((0, 2), 3),
      ((1, 3), 20),
      ((2, 4), 1),
      ((3, 5), 1),
      ((4, 6), 10),
      ((6, 7), 1),
      ((7, 5), 1),
      ((7, 0), 5),
      ((5, 1), 5),
      ((6, 2), 5),
    ];

    let get_neighbor = |id: i64, index: usize| -> Option<Link<i64, i64>> {
      let mut k: usize = 0;
      for ((src, dst), cost) in graph.clone() {
        if src == id {
          if k == index {
            return Some(Link::<i64, i64> {
              neighbor:       dst,
              exact_distance: cost,
              estimate:       (5 - dst).abs(),
            });
          } else {
            k += 1;
          }
        }
      }
      return None;
    };

    let mut open: Vec<Node<i64, i64>> = vec![];
    let mut closed: Vec<Node<i64, i64>> = vec![];

    open.resize(1024, Node::default());
    closed.resize(1024, Node::default());

    let mut state = init(&mut open, 0i64, 5i64, i64::MAX);

    let mut steps = 0;
    let mut neighbor = None;
    loop {
      steps += 1;

      match iteration(&mut open, &mut closed, &mut state, neighbor.clone()) {
        Status::NEIGHBOR(request) => neighbor = get_neighbor(request.node, request.index),
        Status::SUCCESS => break,
        Status::PROGRESS => {},
        _ => assert!(false),
      };
    }

    let mut v: Vec<i64> = vec![];
    v.resize(state.num_closed, 0);
    let n = path(&closed, &state, &mut v);
    v.resize(n, 0);

    assert_eq!(steps, 19);
    assert_eq!(v.len(), 6);
    assert_eq!(v[0], 0);
    assert_eq!(v[1], 2);
    assert_eq!(v[2], 4);
    assert_eq!(v[3], 6);
    assert_eq!(v[4], 7);
    assert_eq!(v[5], 5);
  }
}
