use std::mem::drop;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::thread;

pub use meritrank_core::Weight;

use crate::graph_logic::aug_multi_graph::*;
use crate::utils::log::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Request {
  //  TODO: Factor out types.
  None,
  ReadNodeList,
  ReadNewEdgesFilter(String),
  ReadNodeScore(String, String, String),
  ReadScores(
    String,
    String,
    String,
    bool,
    Weight,
    bool,
    Weight,
    bool,
    u32,
    u32,
  ),
  ReadGraph(String, String, String, bool, u32, u32),
  ReadConnected(String, String),
  ReadEdges(String),
  ReadMutualScores(String, String),
  ReadNeighbors(
    String,
    String,
    String,
    i64,
    String,
    bool,
    Weight,
    bool,
    Weight,
    bool,
    u32,
    u32,
  ),
  WriteReset,
  WriteRecalculateZero,
  WriteRecalculateClustering,
  WriteNewEdgesFilter(String, Vec<u8>),
  WriteFetchNewEdges(String, String),
  WritePutEdge(String, String, String, Weight, i64),
  WriteDeleteEdge(String, String, String, i64),
  WriteDeleteNode(String, String, i64),
  WriteCreateContext(String),
  WriteSetZeroOpinion(String, String, Weight),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestWithId {
  id:   usize,
  data: Request,
}

pub enum Response {
  //  TODO: Factor out types.
  None,
  NodeList(Vec<(String,)>),
  NewEdgesFilter(Vec<u8>),
  NodeScores(Vec<(String, String, Weight, Weight, Cluster, Cluster)>),
  Graph(Vec<(String, String, Weight, Weight, Weight, Cluster, Cluster)>),
  Connections(Vec<(String, String)>),
  Edges(Vec<(String, String, Weight)>),
  NewEdges(Vec<(String, Weight, Weight, Cluster, Cluster)>),
}

#[derive(Default, Clone)]
pub struct InternalState {
  pub cond_run: Arc<Condvar>,
  pub done: Arc<Mutex<bool>>,
  pub graphs: [Arc<Mutex<AugMultiGraph>>; 2],
  pub current_write_graph_index: Arc<Mutex<usize>>,
  pub writing_lock: Arc<Mutex<()>>,
  pub write_requests_queue: Arc<Mutex<Vec<RequestWithId>>>,
  pub next_request_id: Arc<AtomicUsize>,
}

pub struct StateManager {
  pub internal:       InternalState,
  pub writing_thread: thread::JoinHandle<()>,
}

fn is_read_only(request: &Request) -> bool {
  log_trace!();

  match request {
    Request::WriteReset
    | Request::WriteRecalculateZero
    | Request::WriteRecalculateClustering
    | Request::WriteNewEdgesFilter(_, _)
    | Request::WriteFetchNewEdges(_, _)
    | Request::WritePutEdge(_, _, _, _, _)
    | Request::WriteDeleteEdge(_, _, _, _)
    | Request::WriteDeleteNode(_, _, _)
    | Request::WriteCreateContext(_)
    | Request::WriteSetZeroOpinion(_, _, _) => false,
    _ => true,
  }
}

fn perform_request(
  graph: &mut AugMultiGraph,
  request: &Request,
) -> Response {
  log_trace!();

  match request {
    Request::ReadNodeList => Response::NodeList(graph.read_node_list()),

    Request::ReadNewEdgesFilter(src) => {
      Response::NewEdgesFilter(graph.read_new_edges_filter(src))
    },

    Request::ReadNodeScore(context, ego, dst) => {
      Response::NodeScores(graph.read_node_score(context, ego, dst))
    },

    Request::ReadScores(
      context,
      ego,
      kind_str,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count,
    ) => Response::NodeScores(graph.read_scores(
      context,
      ego,
      kind_str,
      *hide_personal,
      *score_lt,
      *score_lte,
      *score_gt,
      *score_gte,
      *index,
      *count,
    )),

    Request::ReadGraph(context, ego, focus, positive_only, index, count) => {
      Response::Graph(graph.read_graph(
        context,
        ego,
        focus,
        *positive_only,
        *index,
        *count,
      ))
    },

    Request::ReadConnected(context, ego) => {
      Response::Connections(graph.read_connected(context, ego))
    },

    Request::ReadEdges(context) => Response::Edges(graph.read_edges(context)),

    Request::ReadMutualScores(context, ego) => {
      Response::NodeScores(graph.read_mutual_scores(context, ego))
    },

    Request::ReadNeighbors(
      context,
      ego,
      focus,
      direction,
      kind_str,
      hide_personal,
      score_lt,
      score_lte,
      score_gt,
      score_gte,
      index,
      count,
    ) => Response::NodeScores(graph.read_neighbors(
      context,
      ego,
      focus,
      *direction,
      kind_str,
      *hide_personal,
      *score_lt,
      *score_lte,
      *score_gt,
      *score_gte,
      *index,
      *count,
    )),

    Request::WriteReset => {
      graph.reset();
      Response::None
    },

    Request::WriteRecalculateZero => {
      graph.write_recalculate_zero();
      Response::None
    },

    Request::WriteRecalculateClustering => {
      graph.write_recalculate_clustering();
      Response::None
    },

    Request::WriteNewEdgesFilter(src, filter_bytes) => {
      graph.write_new_edges_filter(src, filter_bytes);
      Response::None
    },

    Request::WriteFetchNewEdges(src, prefix) => {
      Response::NewEdges(graph.write_fetch_new_edges(src, prefix))
    },

    Request::WritePutEdge(context, src, dst, new_weight, magnitude) => {
      graph.write_put_edge(context, src, dst, *new_weight, *magnitude);
      Response::None
    },

    Request::WriteDeleteEdge(context, src, dst, index) => {
      graph.write_delete_edge(context, src, dst, *index);
      Response::None
    },

    Request::WriteDeleteNode(context, node, index) => {
      graph.write_delete_node(context, node, *index);
      Response::None
    },

    Request::WriteCreateContext(context) => {
      graph.write_create_context(context);
      Response::None
    },

    Request::WriteSetZeroOpinion(context, node, score) => {
      graph.write_set_zero_opinion(context, node, *score);
      Response::None
    },

    _ => {
      log_error!("Unknown request: {:?}", request);
      Response::None
    },
  }
}

fn perform_writing(
  state: &InternalState,
  extra_request: Option<Request>,
) -> Response {
  log_trace!();

  let writing = match state.writing_lock.lock() {
    Ok(x) => x,
    Err(e) => {
      log_error!("{}", e);
      return Response::None;
    },
  };

  //  NOTE: `current_write_graph_index` changes only inside this function guarded by `writing_lock`.

  let write_index = match state.current_write_graph_index.lock() {
    Ok(x) => x,
    Err(e) => {
      log_error!("{}", e);
      return Response::None;
    },
  };

  if *write_index != 0 && *write_index != 1 {
    log_error!("Unexpected current write graph index: {}", write_index);
    return Response::None;
  }

  let mut graph = match state.graphs[*write_index].lock() {
    Ok(x) => x,
    Err(e) => {
      log_error!("{}", e);
      return Response::None;
    },
  };

  drop(write_index); // Make sure to unlock the mutex early.

  let mut queue_performed = vec![]; // We will save performed requests so we can also apply
                                    // them for the other graph copy.

  let mut pick_id = None;
  let mut response = Response::None;

  if let Some(request) = extra_request {
    //  Add extra request to the queue.
    //  We will later return the response for this request.

    let mut queue = match state.write_requests_queue.lock() {
      Ok(x) => x,
      Err(e) => {
        log_error!("{}", e);
        return Response::None;
      },
    };

    let id = state.next_request_id.fetch_add(1, Ordering::Relaxed);

    queue.push(RequestWithId {
      id,
      data: request,
    });

    pick_id = Some(id);
  }

  loop {
    //  Keep performing requests until the queue is empty.

    let mut queue = match state.write_requests_queue.lock() {
      Ok(x) => x,
      Err(e) => {
        log_error!("{}", e);
        return Response::None;
      },
    };

    if let Some(request) = queue.pop() {
      drop(queue); // Make sure to unlock the mutex early.

      let mut need_response = false;

      if let Some(id) = pick_id {
        if id == request.id {
          need_response = true;
        }
      }

      if need_response {
        pick_id = None;
        response = perform_request(&mut graph, &request.data);
      } else {
        perform_request(&mut graph, &request.data);
      }

      queue_performed.push(request); // Save the request to apply in later
                                     // for the other graph copy.
    } else {
      //  The queue is empty - terminate the loop.
      break;
    }
  }

  drop(graph); // Make sure to unlock the mutex early.

  //  Swap the readable and writable graphs.

  let mut write_index = match state.current_write_graph_index.lock() {
    Ok(x) => x,
    Err(e) => {
      log_error!("{}", e);
      return Response::None;
    },
  };

  if *write_index != 0 && *write_index != 1 {
    log_error!("Unexpected current write graph index: {}", write_index);
    return Response::None;
  }

  //  Update the current write graph index.
  *write_index = 1 - *write_index;

  let mut other_graph = match state.graphs[*write_index].lock() {
    Ok(x) => x,
    Err(e) => {
      log_error!("{}", e);
      return Response::None;
    },
  };

  drop(write_index); // Make sure to unlock the mutex early.

  //  Apply requests for the other graph copy.

  for request in queue_performed {
    perform_request(&mut other_graph, &request.data);
  }

  drop(other_graph);
  drop(writing);

  response
}

fn writing_loop(state: InternalState) {
  log_trace!();

  let mut done = match state.done.lock() {
    Ok(x) => x,
    Err(e) => {
      log_error!("{}", e);
      return;
    },
  };

  while !*done {
    drop(done); // Make sure to unlock the mutex early.

    perform_writing(&state, None);

    //  Wait for the state to change.

    //  When the state changes, this thread will be notified
    //  via conditional variable `cond_run`.

    done = match state.done.lock() {
      Ok(x) => x,
      Err(e) => {
        log_error!("{}", e);
        return;
      },
    };

    if *done {
      //  Make sure to terminate the loop if we are done.
      break;
    }

    done = match state.cond_run.wait(done) {
      Ok(x) => x,
      Err(e) => {
        log_error!("{}", e);
        return;
      },
    };
  }
}

fn join(state: StateManager) {
  log_trace!();

  log_verbose!("Joining writing thread.");

  match state.writing_thread.join() {
    Ok(_) => {},
    Err(e) => log_error!("{:?}", e),
  }
}

pub fn init() -> StateManager {
  log_trace!();

  let internal = InternalState::default();

  log_verbose!("Spawning writing thread.");

  let state_cloned = internal.clone();

  let writing_thread = thread::spawn(move || {
    writing_loop(state_cloned);
  });

  StateManager {
    internal,
    writing_thread,
  }
}

pub fn shutdown(state: StateManager) {
  log_trace!();

  match state.internal.done.lock() {
    Ok(mut done) => {
      *done = true;
      state.internal.cond_run.notify_all();
    },
    Err(e) => {
      log_error!("{}", e);
      return;
    },
  }

  join(state);
}

pub fn queue(
  state: &mut InternalState,
  request: Request,
) -> Result<(), ()> {
  log_trace!();

  let mut queue = match state.write_requests_queue.lock() {
    Ok(x) => x,
    Err(e) => {
      log_error!("{}", e);
      return Err(());
    },
  };

  let id = state.next_request_id.fetch_add(1, Ordering::Relaxed);

  queue.push(RequestWithId {
    id,
    data: request,
  });

  state.cond_run.notify_all();
  Ok(())
}

pub fn sync(state: &mut InternalState) {
  log_trace!();

  perform_writing(state, None);
}

pub fn perform(
  state: &mut InternalState,
  request: Request,
) -> Response {
  log_trace!();

  //  NOTE: We apply read requests for read graph copy only, and write requests
  //        for both read and write graph copies.
  //
  //        We assume that read requests have no critical side-effects, so both
  //        copies of the graph are in sync.

  if is_read_only(&request) {
    let write_index = match state.current_write_graph_index.lock() {
      Ok(x) => x,
      Err(e) => {
        log_error!("{}", e);
        return Response::None;
      },
    };

    let read_index = 1 - *write_index;
    drop(write_index); // Make sure to unlock the mutex early.

    let mut graph = match state.graphs[read_index].lock() {
      Ok(x) => x,
      Err(e) => {
        log_error!("{}", e);
        return Response::None;
      },
    };

    perform_request(&mut graph, &request)
  } else {
    perform_writing(state, Some(request))
  }
}
