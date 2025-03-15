---------------------------- MODULE cbc_max ----------------------------

(* An encoding of the conditional consensus protocol based on the maximal value  
   which is proposed by processes. This protocol is described in Fig. 1 with 
   condition C1 in [1].
   
   Mostéfaoui, Achour, et al. "Evaluating the condition-based approach to solve 
   consensus." Dependable Systems and Networks, 2003. Proceedings. 2003 International 
   Conference on. IEEE, 2003.
 
   Igor Konnov, Thanh Hai Tran, Josef Widder, 2016
 
   This file is a subject to the license that is bundled together with this package 
   and can be found in the file LICENSE.
 *)

EXTENDS Integers, FiniteSets, TLC

CONSTANT N, F, T, Values, Bottom

ASSUME 2 * T < N /\ 0 <= F /\ F <= T /\ 0 < N
ASSUME \A v \in Values: v # Bottom

VARIABLES pc, V, v, w, dval, nCrash, sntMsgs, rcvdMsgs

vars == << pc, V, v, w, dval, nCrash, sntMsgs, rcvdMsgs >>

Proc == 1..N

Status == { "BCAST1", "PHS1", "PREP","BCAST2", "PHS2", "DONE", "CRASH", "CHOOSE" }

(* Create a new message *)
Phs1Msg(v_i, i) == [ type |-> "Phs1", value |-> v_i, sndr |-> i ]
Phs2Msg(v_i, w_i, i) == [ type |-> "Phs2", value |-> v_i, wValue |-> w_i, sndr |-> i ]

(* Sets of messages which are broadcasted by processes *)
Msg1s == [ type: {"Phs1"} , value: Values, sndr: Proc ]
Msg2s == [ type: {"Phs2"}, value: Values, wValue: Values, sndr: Proc ]
Msgs == Msg1s \cup Msg2s 

(* Find the maximum value in arr *)
MAX(arr) == CHOOSE maxVal \in Values: /\ (\E p \in Proc: arr[p] = maxVal) 
                                       /\ (\A p \in Proc: maxVal >= arr[p])

(* Line 1 *)
Init ==
  /\ V = [ i \in Proc |-> [ j \in Proc |-> Bottom ] ]
  /\ v \in [ Proc -> Values ]
  /\ pc = [ i \in Proc |-> "BCAST1" ]
  /\ w = [ i \in Proc |-> Bottom ]
  /\ dval = [ i \in Proc |-> Bottom ]
  /\ nCrash = 0 
  /\ sntMsgs = {}
  /\ rcvdMsgs = [ i \in Proc |-> {} ] 
  
(* If there are less than F faulty processes, process i becomes faulty. *)  
Crash(i) ==
  /\ nCrash < F
  /\ pc[i] # "CRASH"
  /\ nCrash' = nCrash + 1
  /\ pc' = [ pc EXCEPT ![i] = "CRASH" ]   
  /\ UNCHANGED << V, w, dval, v, sntMsgs, rcvdMsgs >>

(* Receives a new message *)    
Receive(i) ==
  \E msg \in Msgs :
    /\ pc[i] # "CRASH" 
    /\ msg \in sntMsgs
    /\ msg \notin rcvdMsgs[i]
    /\ rcvdMsgs' = [ rcvdMsgs EXCEPT ![i] = rcvdMsgs[i] \cup { msg } ]         
    /\ LET j == msg.sndr
       IN V' = [ V EXCEPT ![i][j] = IF \/ /\ pc[i] = "PHS1" 
                                          /\ msg.type = "Phs1"
                                       \/ /\ pc[i] = "PHS2" 
                                          /\ msg.type = "Phs2"
                                    THEN msg.value
                                    ELSE V[i][j] ] 
    /\ UNCHANGED << pc, v, w, dval, nCrash, sntMsgs, V >>               
   
(* Broadcasts PHASE1(v_i, i) *)  
BcastPhs1(i) ==  
  /\ pc[i] = "BCAST1"
  /\ pc' = [ pc EXCEPT ![i] = "PHS1" ] 
  /\ sntMsgs' = sntMsgs \cup { Phs1Msg(v[i], i) } 
  /\ UNCHANGED << V, v, w, dval, nCrash, rcvdMsgs >>  
   
(* If a process received PHASE1(_, _) from at least N - F processes, it is ready
   to update its view and to make an estimation.
 *)   
Phs1(i) == 
  /\ pc[i] = "PHS1"
  /\ pc' = [ pc EXCEPT ![i] = "BCAST2" ]
  /\ Cardinality({ m \in rcvdMsgs[i]: m.type = "Phs1" }) >= N - T
  /\ w' = [ w EXCEPT ![i] = MAX(V[i]) ]
  /\ UNCHANGED << v, dval, nCrash, sntMsgs, rcvdMsgs, V >>                
  
   
(* A process broadcasts its estimated value. *)   
BcastPhs2(i) ==
  /\ pc[i] = "BCAST2"
  /\ pc' = [ pc EXCEPT ![i] = "PHS2" ] 
  /\ sntMsgs' = sntMsgs \cup { Phs2Msg(v[i], w[i], i) } 
  /\ UNCHANGED << V, v, w, dval, nCrash, rcvdMsgs >>  
  
(* If a process receives a new PHASE2, it updates its local view. If the expected 
   value w in the message is also one from the majority, it decides w. If the input 
   vector does not belong to the condition and no process crashes, V_i eventually 
   becomes the "full" input vector and process i deterministically decide. If all 
   PHASE2 messages has received, process i moves to step Choose.
 *)  
Phs2(i) ==  
  /\ pc[i] = "PHS2"
  /\ \/ \E v0 \in Values:  
            /\ Cardinality( { m \in rcvdMsgs[i]: m.type = "Phs2" /\ m.wValue = v0 } )  >= N - T  
            /\ dval' = [ dval EXCEPT ![i] = v0 ]
            /\ pc' = [ pc EXCEPT ![i] = "DONE" ]             
            /\ UNCHANGED << v, w, nCrash, sntMsgs, rcvdMsgs, V >>
     \/ /\ \A j \in Proc: \E m \in rcvdMsgs[i] : m.type = "Phs2" /\ m.sndr = j
        /\ pc' = [ pc EXCEPT ![i] = "CHOOSE"]
        /\ UNCHANGED << v, w, nCrash, sntMsgs, dval, rcvdMsgs, V >>

(* Process i has received all PHASE2 messages and therefore, it can deterministically
   choose a value appearing in V[i]. 
 *)
Choose(i) ==       
  /\ pc[i] = "CHOOSE"
  /\ dval' = [ dval EXCEPT ![i] = (CHOOSE tV \in Values: (\E j \in Proc: tV = V[i][j])) ]
  /\ pc' = [ pc EXCEPT ![i] = "DONE" ] 
  /\ UNCHANGED << V, v, w, nCrash, sntMsgs, rcvdMsgs >> 
      
Next == \E i \in Proc: \/ Crash(i)      
                       \/ Receive(i)      
                       \/ BcastPhs1(i)      
                       \/ Phs1(i)      
                       \/ BcastPhs2(i)      
                       \/ Phs2(i)
                       \/ Choose(i)
                       \/ /\ \A p \in Proc : pc[p] = "CRASH" \/ pc[p] = "DONE"
                          /\ UNCHANGED vars
      
Spec == Init /\ [][Next]_vars 
       /\ WF_vars(\E i \in Proc: \/ Receive(i)      
                                 \/ BcastPhs1(i)      
                                 \/ Phs1(i)      
                                 \/ BcastPhs2(i)      
                                 \/ Phs2(i)  
                                 \/ Choose(i))

TypeOK ==
  /\ V \in [ Proc -> [ Proc -> { Bottom } \cup Values ] ]
  /\ v \in [ Proc -> Values ] 
  /\ pc \in [ Proc -> Status ]
  /\ w \in [ Proc -> { Bottom } \cup Values ]
  /\ dval \in [ Proc -> { Bottom } \cup Values ]
  /\ nCrash \in 0 .. F 
  /\ sntMsgs \in SUBSET Msgs
  /\ rcvdMsgs \in [ Proc -> SUBSET Msgs ] 
  
  
(* If a process decides v, then v was proposed by some process. *)
Validity == (\A i \in Proc: dval[i] # Bottom => (\E j \in Proc: dval[i] = v[j]))

(* No two processes decide differently. *)
Agreement == \A i, j \in Proc: (dval[i] # Bottom /\ dval[j] # Bottom) => dval[i] = dval[j]

(* Every correct process eventually decides on some values. *)
Termination == <>(\A i \in Proc: pc[i] = "CRASH" \/ pc[i] = "DONE")

(* At least F + 1 processes initialize with the greatest value MAX(v). *)
Condition1 == Cardinality({ j \in Proc: v[j] = MAX(v)}) > F
  
(* If the input vector satisfies the Condition1, the algorithm terminates. *)  
RealTermination == Condition1 => Termination  
    
=============================================================================
\* Modification History
\* Last modified Mon Jul 09 16:15:38 CEST 2018 by tthai
\* Last modified Mon Jul 09 13:27:23 CEST 2018 by tran
\* Created Tue Nov 22 10:32:35 CET 2016 by tran
