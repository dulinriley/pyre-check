(*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *)

(* Issue: implements the logic that matches sources against sinks, using the
 * current set of rules, and convert them into issues.
 * It also defines a handle that uniquely represents issues.
 *)

open Core
open Ast
open Domains
open Interprocedural
open Pyre

module Flow = struct
  type t = {
    source_taint: ForwardTaint.t;
    sink_taint: BackwardTaint.t;
  }
  [@@deriving show]

  let bottom = { source_taint = ForwardTaint.bottom; sink_taint = BackwardTaint.bottom }

  let is_bottom { source_taint; sink_taint } =
    ForwardTaint.is_bottom source_taint || BackwardTaint.is_bottom sink_taint


  let join
      { source_taint = left_source_taint; sink_taint = left_sink_taint }
      { source_taint = right_source_taint; sink_taint = right_sink_taint }
    =
    {
      source_taint = ForwardTaint.join left_source_taint right_source_taint;
      sink_taint = BackwardTaint.join left_sink_taint right_sink_taint;
    }
end

module LocationSet = Stdlib.Set.Make (Location.WithModule)

type t = {
  flow: Flow.t;
  handle: IssueHandle.t;
  locations: LocationSet.t;
  define: Statement.Define.t Node.t;
}

let join
    { flow = flow_left; handle; locations = locations_left; define }
    { flow = flow_right; handle = _; locations = locations_right; define = _ }
  =
  {
    flow = Flow.join flow_left flow_right;
    handle;
    locations = LocationSet.union locations_left locations_right;
    define;
  }


let canonical_location { locations; _ } =
  Option.value_exn ~message:"issue has no location" (LocationSet.min_elt_opt locations)


(* Define how to group issue candidates for a given function. *)
module CandidateKey = struct
  module T = struct
    type t = {
      location: Location.WithModule.t;
      sink_handle: IssueHandle.Sink.t;
    }
    [@@deriving compare, sexp, hash]
  end

  include T
  include Hashable.Make (T)
end

module Candidate = struct
  type t = {
    flows: Flow.t list;
    key: CandidateKey.t;
  }

  let join { flows = left_flows; key } { flows = right_flows; _ } =
    { flows = List.rev_append left_flows right_flows; key }
end

(* Compute all flows from paths in ~source tree to corresponding paths in ~sink tree, while avoiding
   duplication as much as possible.

   Strategy:

   Let F and B for forward and backward taint respectively. For each path p in B from the root to
   some node with non-empty taint T, we match T with the join of taint in the upward and downward
   closure from node at path p in F. *)
let generate_source_sink_matches ~location ~sink_handle ~source_tree ~sink_tree =
  let make_source_sink_matches (path, sink_taint) matches =
    let source_taint =
      ForwardState.Tree.read path source_tree
      |> ForwardState.Tree.collapse ~breadcrumbs:(Features.issue_broadening_set ())
    in
    if ForwardTaint.is_bottom source_taint then
      matches
    else
      { Flow.source_taint; sink_taint } :: matches
  in
  let flows =
    if ForwardState.Tree.is_empty source_tree then
      []
    else
      BackwardState.Tree.fold BackwardState.Tree.Path ~init:[] ~f:make_source_sink_matches sink_tree
  in
  { Candidate.flows; key = { location; sink_handle } }


module PartitionedFlow = struct
  type t = {
    source_partition: (Sources.t, ForwardTaint.t) Map.Poly.t;
    sink_partition: (Sinks.t, BackwardTaint.t) Map.Poly.t;
  }
end

let generate_issues
    ~taint_configuration
    ~define
    { Candidate.flows; key = { location; sink_handle } }
  =
  let partitions =
    let partition { Flow.source_taint; sink_taint } =
      {
        PartitionedFlow.source_partition =
          ForwardTaint.partition ForwardTaint.kind By source_taint ~f:(fun kind ->
              kind |> Sources.discard_transforms |> Sources.discard_subkind);
        sink_partition =
          BackwardTaint.partition BackwardTaint.kind By sink_taint ~f:(fun kind ->
              kind |> Sinks.discard_transforms |> Sinks.discard_subkind);
      }
    in
    List.map flows ~f:partition
  in
  let apply_rule_on_flow
      { Rule.sources; sinks; transforms; _ }
      { PartitionedFlow.source_partition; sink_partition }
    =
    let add_source_taint source_taint source =
      match Map.Poly.find source_partition (Sources.discard_subkind source) with
      | Some taint -> ForwardTaint.join source_taint taint
      | None -> source_taint
    in
    let add_sink_taint sink_taint sink =
      match Map.Poly.find sink_partition (Sinks.discard_subkind sink) with
      | Some taint -> BackwardTaint.join sink_taint taint
      | None -> sink_taint
    in
    let source_taint = List.fold sources ~f:add_source_taint ~init:ForwardTaint.bottom in
    let sink_taint = List.fold sinks ~f:add_sink_taint ~init:BackwardTaint.bottom in

    let rec apply_sanitizers
        ?(previous_sanitized_sources = Sources.Set.empty)
        ?(previous_sanitized_sinks = Sinks.Set.empty)
        ?(previous_single_base_source = None)
        ?(previous_single_base_sink = None)
        { Flow.source_taint; sink_taint }
      =
      (* This needs a fixpoint since refining sinks might sanitize more sources etc.
       * For instance:
       * Sources: {Not[X]@A, Not[X]:Not[Y]@C}
       * Sinks: {X, Not[A]@Y}
       * After one iteration, we still have {Not[X]:Not[Y]@C} and {Not[A]@Y},
       * which can be refined further to an invalid flow.
       *)
      let gather_sanitized_sinks kind sofar =
        let sanitized =
          kind
          |> Sources.extract_sanitize_transforms
          |> (fun { sinks; _ } -> sinks)
          |> Sinks.extract_sanitized_sinks_from_transforms
        in
        match sofar with
        | None -> Some sanitized
        | Some sofar -> Some (Sinks.Set.inter sofar sanitized)
      in
      let sanitized_sinks =
        ForwardTaint.fold ForwardTaint.kind ~init:None ~f:gather_sanitized_sinks source_taint
        |> Option.value ~default:Sinks.Set.empty
      in
      let sink_taint = BackwardTaint.sanitize_taint_kinds sanitized_sinks sink_taint in

      let gather_sanitized_sources kind sofar =
        let sanitized =
          kind
          |> Sinks.extract_sanitize_transforms
          |> (fun { sources; _ } -> sources)
          |> Sources.extract_sanitized_sources_from_transforms
        in
        match sofar with
        | None -> Some sanitized
        | Some sofar -> Some (Sources.Set.inter sofar sanitized)
      in
      let sanitized_sources =
        BackwardTaint.fold BackwardTaint.kind ~init:None ~f:gather_sanitized_sources sink_taint
        |> Option.value ~default:Sources.Set.empty
      in
      let source_taint = ForwardTaint.sanitize_taint_kinds sanitized_sources source_taint in

      (* If all sources have the same base, we can remove sink flows that sanitize
       * that base (and vice versa). *)
      let gather_base_sources kind sofar =
        Sources.Set.add
          (kind |> Sources.discard_sanitize_transforms |> Sources.discard_subkind)
          sofar
      in
      let single_base_source =
        ForwardTaint.fold
          ForwardTaint.kind
          ~init:Sources.Set.empty
          ~f:gather_base_sources
          source_taint
        |> Sources.Set.as_singleton
      in
      let sink_taint =
        match single_base_source with
        | Some (Sources.NamedSource source) ->
            let sanitize_transforms =
              SanitizeTransform.Source.Named source
              |> SanitizeTransform.SourceSet.singleton
              |> SanitizeTransformSet.from_sources
            in
            BackwardTaint.transform
              BackwardTaint.kind
              Filter
              ~f:(fun kind -> not (Sinks.contains_sanitize_transforms kind sanitize_transforms))
              sink_taint
        | _ -> sink_taint
      in

      let gather_base_sinks kind sofar =
        Sinks.Set.add (kind |> Sinks.discard_sanitize_transforms |> Sinks.discard_subkind) sofar
      in
      let single_base_sink =
        BackwardTaint.fold BackwardTaint.kind ~init:Sinks.Set.empty ~f:gather_base_sinks sink_taint
        |> Sinks.Set.as_singleton
      in
      let source_taint =
        match single_base_sink with
        | Some (Sinks.NamedSink sink) ->
            let sanitize_transforms =
              SanitizeTransform.Sink.Named sink
              |> SanitizeTransform.SinkSet.singleton
              |> SanitizeTransformSet.from_sinks
            in
            ForwardTaint.transform
              ForwardTaint.kind
              Filter
              ~f:(fun kind -> not (Sources.contains_sanitize_transforms kind sanitize_transforms))
              source_taint
        | _ -> source_taint
      in

      if
        Sources.Set.equal sanitized_sources previous_sanitized_sources
        && Sinks.Set.equal sanitized_sinks previous_sanitized_sinks
        && Option.equal Sources.equal single_base_source previous_single_base_source
        && Option.equal Sinks.equal single_base_sink previous_single_base_sink
      then
        { Flow.source_taint; sink_taint }
      else
        apply_sanitizers
          ~previous_sanitized_sources:sanitized_sources
          ~previous_sanitized_sinks:sanitized_sinks
          ~previous_single_base_source:single_base_source
          ~previous_single_base_sink:single_base_sink
          { source_taint; sink_taint }
    in
    let apply_transforms { Flow.source_taint; sink_taint } =
      let taint_by_source_transforms =
        ForwardTaint.partition ForwardTaint.kind By source_taint ~f:Sources.get_named_transforms
      in
      let taint_by_sink_transforms =
        BackwardTaint.partition BackwardTaint.kind By sink_taint ~f:Sinks.get_named_transforms
      in
      let find_flow source_transforms sink_transforms =
        Map.Poly.find taint_by_source_transforms source_transforms
        >>= fun source_taint ->
        Map.Poly.find taint_by_sink_transforms sink_transforms
        >>| fun sink_taint -> { Flow.source_taint; sink_taint }
      in
      let add_and_sanitize_flow sofar (source_transforms, sink_transforms) =
        find_flow source_transforms sink_transforms
        >>| apply_sanitizers
        |> Option.value_map ~default:sofar ~f:(Flow.join sofar)
      in
      Rule.transform_splits transforms |> List.fold ~init:Flow.bottom ~f:add_and_sanitize_flow
    in
    let partition_flow = apply_transforms { source_taint; sink_taint } in
    if Flow.is_bottom partition_flow then
      None
    else
      Some partition_flow
  in
  let apply_rule_separate_access_path issues_so_far (rule : Rule.t) =
    let fold_partitions issues candidate =
      match apply_rule_on_flow rule candidate with
      | Some flow ->
          {
            flow;
            handle = { code = rule.code; callable = Target.create define; sink = sink_handle };
            locations = LocationSet.singleton location;
            define;
          }
          :: issues
      | None -> issues
    in
    List.fold partitions ~init:issues_so_far ~f:fold_partitions
  in
  let apply_rule_merge_access_path rule =
    let fold_partitions flow_so_far candidate =
      match apply_rule_on_flow rule candidate with
      | Some flow -> Flow.join flow_so_far flow
      | None -> flow_so_far
    in
    let flow =
      List.fold
        partitions
        ~init:{ Flow.source_taint = ForwardTaint.bottom; sink_taint = BackwardTaint.bottom }
        ~f:fold_partitions
    in
    if Flow.is_bottom flow then
      None
    else
      Some
        {
          flow;
          handle = { code = rule.code; callable = Target.create define; sink = sink_handle };
          locations = LocationSet.singleton location;
          define;
        }
  in
  let group_by_handle map issue =
    (* SAPP invariant: There should be a single issue per issue handle.
     * The configuration might have multiple rules with the same code due to
     * multi source-sink rules, hence we need to merge issues here. *)
    let update = function
      | None -> issue
      | Some previous_issue -> join previous_issue issue
    in
    IssueHandle.Map.update map issue.handle ~f:update
  in
  if taint_configuration.TaintConfiguration.Heap.lineage_analysis then
    (* Create different issues for same access path, e.g, Issue{[a] -> [b]}, Issue {[c] -> [d]}. *)
    (* Note that this breaks a SAPP invariant because there might be multiple issues with the same
       handle. This is fine because in that configuration we do not use SAPP. *)
    List.fold taint_configuration.rules ~init:[] ~f:apply_rule_separate_access_path
  else (* Create single issue for same access path, e.g, Issue{[a],[c] -> [b], [d]}. *)
    List.filter_map ~f:apply_rule_merge_access_path taint_configuration.rules
    |> List.fold ~init:IssueHandle.Map.empty ~f:group_by_handle
    |> IssueHandle.Map.data


(* A map from triggered sink kinds (which is a string) to the triggered sink taints to propagate in
   the backward analysis. For a multi-source rule, triggered sinks do not mean we have found the
   issue, because the other sources are still missing. *)
module TriggeredSinkHashMap = struct
  module Hashable = Core.Hashable.Make (String)
  module HashMap = Hashable.Table

  type t = BackwardTaint.t HashMap.t

  let create () = HashMap.create ()

  let is_empty map = HashMap.is_empty map

  let convert_to_key partial_sink = Sinks.show_partial_sink partial_sink

  let mem map partial_sink = HashMap.mem map (convert_to_key partial_sink)

  let add
      map
      ~triggered_sink
      ~extra_trace:({ ExtraTraceFirstHop.call_info; _ } as extra_trace)
      ~issue_handles
    =
    let add_extra_traces_and_handles triggered_sink =
      let triggered_sink =
        if CallInfo.show_as_extra_trace call_info then
          (* Sources, which have been matched with sinks and thus cause the creation of the
             triggered sinks, as well as the corresponding call_infos. This pair constitutes the
             first hops of the matched source traces *)
          BackwardTaint.transform
            ExtraTraceFirstHop.Set.Self
            Map
            ~f:(ExtraTraceFirstHop.Set.add extra_trace)
            triggered_sink
        else
          triggered_sink
      in
      (* Handles of the issues that are created when creating the triggered sinks *)
      BackwardTaint.transform
        Domains.IssueHandleSet.Self
        Map
        ~f:(Domains.IssueHandleSet.join issue_handles)
        triggered_sink
    in
    let update = function
      | Some triggered_sink -> add_extra_traces_and_handles triggered_sink
      | None ->
          BackwardTaint.singleton
            CallInfo.declaration
            (Sinks.TriggeredPartialSink triggered_sink)
            Frame.initial
          |> add_extra_traces_and_handles
    in
    HashMap.update map (convert_to_key triggered_sink) ~f:update


  let find map partial_sink = HashMap.find map (convert_to_key partial_sink)
end

(* A map from locations to a set of triggered sinks.
 * This is used to store triggered sinks found in the forward analysis,
 * and propagate them up in the backward analysis. *)
module TriggeredSinkLocationMap = struct
  type t = BackwardState.t Location.Table.t

  let create () = Location.Table.create ()

  let add map ~location ~taint =
    Hashtbl.update map location ~f:(function
        | Some existing -> BackwardState.join existing taint
        | None -> taint)


  let get map ~location = Hashtbl.find map location |> Option.value ~default:BackwardState.bottom
end

let compute_triggered_flows
    ~taint_configuration
    ~triggered_sinks_for_call
    ~location
    ~sink_handle
    ~source_tree
    ~sink_tree
    ~define
  =
  let partial_sinks =
    BackwardState.Tree.fold
      BackwardTaint.kind
      ~f:(fun sink sofar ->
        match Sinks.extract_partial_sink sink with
        | Some partial_sink -> partial_sink :: sofar
        | None -> sofar)
      ~init:[]
      sink_tree
  in
  let call_infos_and_sources =
    if List.is_empty partial_sinks then
      []
    else
      ForwardState.Tree.reduce
        ForwardTaint.kind
        ~using:(Context (ForwardTaint.call_info, Acc))
        ~f:(fun call_info source sofar -> (call_info, source) :: sofar)
        ~init:[]
        source_tree
  in
  let check_source_sink_flows ~candidates ~call_info ~source ~partial_sink =
    TaintConfiguration.get_triggered_sink taint_configuration ~partial_sink ~source
    |> function
    | Some (Sinks.TriggeredPartialSink triggered_sink) ->
        let ({ Candidate.flows; _ } as candidate) =
          generate_source_sink_matches
            ~location
            ~sink_handle
            ~source_tree
            ~sink_tree:
              (BackwardState.Tree.create_leaf
                 (BackwardTaint.singleton
                    (CallInfo.Origin location)
                    (Sinks.TriggeredPartialSink partial_sink)
                    Frame.initial))
        in
        if List.is_empty flows then
          candidates
        else
          (* For a multi-source rule, the candidate could be the first issue that is discovered, or
             the second. We consider both situations as valid issues here, but after the global
             fixpoint computation is done, we will remove the non-main issue. *)
          let extra_trace =
            { ExtraTraceFirstHop.call_info; leaf_kind = Source source; message = None }
          in
          let issues = generate_issues ~taint_configuration ~define candidate in
          let issue_handles =
            List.fold issues ~init:Domains.IssueHandleSet.bottom ~f:(fun so_far issue ->
                Domains.IssueHandleSet.add issue.handle so_far)
          in
          TriggeredSinkHashMap.add
            triggered_sinks_for_call
            ~triggered_sink
            ~extra_trace
            ~issue_handles;
          if TriggeredSinkHashMap.mem triggered_sinks_for_call partial_sink then
            (* We have both pairs, let's check the flow directly for this sink being triggered. *)
            candidate :: candidates
          else
            candidates
    | _ -> candidates
  in
  let check_sink_flows candidates partial_sink =
    List.fold
      ~f:(fun candidates (call_info, source) ->
        check_source_sink_flows ~candidates ~call_info ~source ~partial_sink)
      ~init:candidates
      call_infos_and_sources
  in
  List.fold ~f:check_sink_flows ~init:[] partial_sinks


module Candidates = struct
  type issue = t

  type t = Candidate.t CandidateKey.Table.t

  let create () = CandidateKey.Table.create ()

  let add_candidate candidates ({ Candidate.key; _ } as candidate) =
    CandidateKey.Table.update candidates key ~f:(function
        | None -> candidate
        | Some current_candidate -> Candidate.join current_candidate candidate)


  (* Check for issues in flows from the `source_tree` to the `sink_tree`, updating
   * issue `candidates`. *)
  let check_flow candidates ~location ~sink_handle ~source_tree ~sink_tree =
    generate_source_sink_matches ~location ~sink_handle ~source_tree ~sink_tree
    |> add_candidate candidates


  (* Check for issues for combined source rules.
   * For flows where both sources are present, this adds the flow to issue `candidates`.
   * If only one source is present, this creates a triggered sink in `triggered_sinks_for_call`.
   *)
  let check_triggered_flows
      candidates
      ~taint_configuration
      ~triggered_sinks_for_call
      ~location
      ~sink_handle
      ~source_tree
      ~sink_tree
      ~define
    =
    let new_candidates =
      compute_triggered_flows
        ~taint_configuration
        ~triggered_sinks_for_call
        ~sink_handle
        ~location
        ~source_tree
        ~sink_tree
        ~define
    in
    List.iter new_candidates ~f:(add_candidate candidates)


  let generate_issues candidates ~taint_configuration ~define =
    let accumulate ~key:_ ~data:candidate issues =
      let new_issues = generate_issues ~taint_configuration ~define candidate in
      List.rev_append new_issues issues
    in
    CandidateKey.Table.fold candidates ~f:accumulate ~init:[]
end

type features = {
  breadcrumbs: Features.BreadcrumbSet.t;
  first_indices: Features.FirstIndexSet.t;
  first_fields: Features.FirstFieldSet.t;
}

let get_issue_features { Flow.source_taint; sink_taint } =
  let breadcrumbs =
    let source_breadcrumbs = ForwardTaint.joined_breadcrumbs source_taint in
    let sink_breadcrumbs = BackwardTaint.joined_breadcrumbs sink_taint in
    Features.BreadcrumbSet.sequence_join source_breadcrumbs sink_breadcrumbs
  in
  let first_indices =
    let source_indices = ForwardTaint.first_indices source_taint in
    let sink_indices = BackwardTaint.first_indices sink_taint in
    Features.FirstIndexSet.join source_indices sink_indices
  in
  let first_fields =
    let source_fields = ForwardTaint.first_fields source_taint in
    let sink_fields = BackwardTaint.first_fields sink_taint in
    Features.FirstFieldSet.join source_fields sink_fields
  in

  { breadcrumbs; first_indices; first_fields }


let sinks_regexp = Str.regexp_string "{$sinks}"

let sources_regexp = Str.regexp_string "{$sources}"

let transforms_regexp = Str.regexp_string "{$transforms}"

let get_name_and_detailed_message
    ~taint_configuration:{ TaintConfiguration.Heap.rules; _ }
    { flow; handle = { code; _ }; _ }
  =
  match List.find ~f:(fun { code = rule_code; _ } -> code = rule_code) rules with
  | None -> failwith "issue with code that has no rule"
  | Some { name; message_format; transforms; _ } ->
      let sources =
        Domains.ForwardTaint.kinds flow.source_taint
        |> List.map ~f:Sources.discard_transforms
        |> List.dedup_and_sort ~compare:Sources.compare
        |> List.map ~f:Sources.show
        |> String.concat ~sep:", "
      in
      let sinks =
        Domains.BackwardTaint.kinds flow.sink_taint
        |> List.map ~f:Sinks.discard_transforms
        |> List.dedup_and_sort ~compare:Sinks.compare
        |> List.map ~f:Sinks.show
        |> String.concat ~sep:", "
      in
      let transforms = List.map transforms ~f:TaintTransform.show |> String.concat ~sep:", " in
      let message =
        Str.global_replace sources_regexp sources message_format
        |> Str.global_replace sinks_regexp sinks
        |> Str.global_replace transforms_regexp transforms
      in
      name, message


let to_error
    ~taint_configuration:({ TaintConfiguration.Heap.rules; _ } as taint_configuration)
    ({ handle = { code; _ }; define; _ } as issue)
  =
  match List.find ~f:(fun { code = rule_code; _ } -> code = rule_code) rules with
  | None -> failwith "issue with code that has no rule"
  | Some _ ->
      let name, detail = get_name_and_detailed_message ~taint_configuration issue in
      let kind = { Error.name; messages = [detail]; code } in
      let location = canonical_location issue in
      Error.create ~location ~define ~kind


let to_json ~taint_configuration ~expand_overrides ~is_valid_callee ~filename_lookup issue =
  let callable_name = Target.external_name issue.handle.callable in
  let _, message = get_name_and_detailed_message ~taint_configuration issue in
  let source_traces =
    Domains.ForwardTaint.to_json
      ~expand_overrides
      ~is_valid_callee
      ~filename_lookup:(Some filename_lookup)
      issue.flow.source_taint
  in
  let sink_traces =
    Domains.BackwardTaint.to_json
      ~expand_overrides
      ~is_valid_callee
      ~filename_lookup:(Some filename_lookup)
      issue.flow.sink_taint
  in
  let features = get_issue_features issue.flow in
  let json_features =
    let get_feature_json { Abstract.OverUnderSetDomain.element; in_under } breadcrumbs =
      let element = Features.BreadcrumbInterned.unintern element in
      let breadcrumb_json = Features.Breadcrumb.to_json element ~on_all_paths:in_under in
      breadcrumb_json :: breadcrumbs
    in
    Features.BreadcrumbSet.fold
      Features.BreadcrumbSet.ElementAndUnder
      ~f:get_feature_json
      ~init:[]
      features.breadcrumbs
  in
  let json_features =
    List.concat
      [
        features.first_indices
        |> Features.FirstIndexSet.elements
        |> List.map ~f:Features.FirstIndexInterned.unintern
        |> Features.FirstIndex.to_json;
        features.first_fields
        |> Features.FirstFieldSet.elements
        |> List.map ~f:Features.FirstFieldInterned.unintern
        |> Features.FirstField.to_json;
        json_features;
      ]
  in
  let traces : Yojson.Safe.t =
    `List
      [
        `Assoc ["name", `String "forward"; "roots", source_traces];
        `Assoc ["name", `String "backward"; "roots", sink_traces];
      ]
  in
  let {
    Location.WithPath.path;
    start = { line; column = start_column };
    stop = { column = stop_column; _ };
  }
    =
    canonical_location issue |> Location.WithModule.instantiate ~lookup:filename_lookup
  in
  let callable_line = Ast.(Location.line issue.define.location) in
  let sink_handle = IssueHandle.Sink.to_json issue.handle.sink in
  let master_handle = IssueHandle.master_handle issue.handle in
  `Assoc
    [
      "callable", `String callable_name;
      "callable_line", `Int callable_line;
      "code", `Int issue.handle.code;
      "line", `Int line;
      "start", `Int start_column;
      "end", `Int stop_column;
      "filename", `String path;
      "message", `String message;
      "traces", traces;
      "features", `List json_features;
      "sink_handle", sink_handle;
      "master_handle", `String master_handle;
    ]
