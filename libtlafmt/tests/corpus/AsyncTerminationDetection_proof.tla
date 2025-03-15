---------------------- MODULE AsyncTerminationDetection_proof ---------------------
(*********************************************************************************)
(* Proofs about the high-level specification of termination detection.           *)
(*                                                                               *)
(* Please note that the liveness proof below requires building tlapm from source *)
(* using the branch available at                                                 *)
(* https://github.com/tlaplus/tlapm/tree/updated_enabled_cdot.                   *)
(* Running the standard distribution of TLAPS on this module will result in an   *)
(* error message about an unknown proof directive.                               *)
(*********************************************************************************)

EXTENDS AsyncTerminationDetection, TLAPS

LEMMA TypeCorrect == Init /\ [][Next]_vars => []TypeOK
<1>. USE NAssumption DEF Node, TypeOK
<1>1. Init => TypeOK
  BY Isa DEF Init, terminated
<1>2. TypeOK /\ [Next]_vars => TypeOK'
 <2> SUFFICES ASSUME TypeOK,
                     [Next]_vars
              PROVE  TypeOK'
   OBVIOUS
 <2>1. CASE DetectTermination
   BY <2>1 DEF DetectTermination
 <2>2. ASSUME NEW i \in Node,
              NEW j \in Node,
              Terminate(i)
       PROVE  TypeOK'
   BY <2>2, Zenon DEF Terminate, terminated
 <2>3. ASSUME NEW i \in Node,
              NEW j \in Node,
              RcvMsg(i)
       PROVE  TypeOK'
   BY <2>3 DEF RcvMsg
 <2>4. ASSUME NEW i \in Node,
              NEW j \in Node,
              SendMsg(i, j)
       PROVE  TypeOK'
   BY <2>4 DEF SendMsg
 <2>5. CASE UNCHANGED vars
   BY <2>5 DEF vars
 <2>6. QED
   BY <2>1, <2>2, <2>3, <2>4, <2>5 DEF Next
<1>. QED BY <1>1, <1>2, PTL

(***************************************************************************)
(* Proofs of safety and stability.                                         *)
(***************************************************************************)
THEOREM Safety == Init /\ [][Next]_vars => []Safe
<1>. USE DEF terminated, TypeOK, Safe
<1>1. Init => Safe
  BY Zenon DEF Init
<1>2. TypeOK /\ Safe /\ [Next]_vars => Safe'
  <2> SUFFICES ASSUME TypeOK, Safe, [Next]_vars
               PROVE  Safe'
    OBVIOUS
  <2>1. CASE DetectTermination
    BY <2>1 DEF DetectTermination
  <2>2. ASSUME NEW i \in Node, Terminate(i)
        PROVE  Safe'
    BY <2>2, Zenon DEF Terminate
  <2>3. ASSUME NEW i \in Node, RcvMsg(i)
        PROVE  Safe'
    BY <2>3 DEF RcvMsg
  <2>4. ASSUME NEW i \in Node, NEW j \in Node, SendMsg(i, j)
        PROVE  Safe'
    BY <2>4 DEF SendMsg
  <2>5. CASE UNCHANGED vars
    BY <2>5 DEF vars
  <2>. QED
    BY <2>1, <2>2, <2>3, <2>4, <2>5 DEF Next
<1>. QED
  BY <1>1, <1>2, TypeCorrect, PTL

THEOREM Stability == Init /\ [][Next]_vars => Quiescence
<1>1. TypeOK /\ terminated /\ [Next]_vars => terminated'
    BY Isa DEF TypeOK, terminated, Next, DetectTermination, Terminate, RcvMsg, SendMsg, vars
<1>. QED  BY <1>1, TypeCorrect, PTL DEF Quiescence

(***************************************************************************)
(* Proofs of liveness.                                                     *)
(***************************************************************************)

(***************************************************************************)
(* We first reduce the enabledness condition that appears in the fairness  *)
(* hypothesis to a standard state predicate.                               *)
(***************************************************************************)
LEMMA EnabledDT == 
  ASSUME TypeOK 
  PROVE  (ENABLED <<DetectTermination>>_vars) <=> (terminated /\ ~ terminationDetected)
<1>1. ASSUME terminated, ~ terminationDetected
      PROVE  ENABLED <<DetectTermination>>_vars
  <2>1. <<DetectTermination>>_vars <=> DetectTermination
    BY <1>1 DEF TypeOK, terminated, DetectTermination, vars
  <2>2. (ENABLED <<DetectTermination>>_vars) <=> (ENABLED DetectTermination)
    BY <2>1, ENABLEDrules
  <2>3. ENABLED UNCHANGED <<active, pending>>
    BY ExpandENABLED
  <2>4. ENABLED DetectTermination
    BY <1>1, <2>3, ENABLEDrewrites DEF DetectTermination
  <2>. QED  BY <2>2, <2>4
<1>2. ASSUME ENABLED <<DetectTermination>>_vars
      PROVE  terminated /\ ~ terminationDetected
  BY <1>2, ExpandENABLED, Zenon DEF DetectTermination, terminated, vars
<1>. QED  BY <1>1, <1>2

THEOREM Liveness == Spec => Live
<1>. DEFINE P == terminated /\ ~ terminationDetected
            Q == terminationDetected
<1>1. TypeOK /\ P /\ [Next]_vars => P' \/ Q'
  BY Isa DEF TypeOK, terminated, Next, vars, Terminate, SendMsg, RcvMsg, DetectTermination
<1>2. TypeOK /\ P /\ <<DetectTermination>>_vars => Q'
  BY DEF DetectTermination
<1>3. TypeOK /\ P => ENABLED <<DetectTermination>>_vars
  BY EnabledDT
<1>. QED  BY <1>1, <1>2, <1>3, TypeCorrect, PTL DEF Spec, Live


=============================================================================
\* Modification History
\* Last modified Wed Jun 29 09:28:02 CEST 2022 by merz
\* Last modified Wed Jun 02 14:19:14 PDT 2021 by markus
\* Created Sun Jan 10 15:19:20 CET 2021 by merz
