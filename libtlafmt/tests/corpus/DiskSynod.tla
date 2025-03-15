----------------------------- MODULE DiskSynod -----------------------------

EXTENDS Synod, FiniteSets

CONSTANTS Ballot(_), Disk, IsMajority(_)
ASSUME /\ \A p \in Proc : /\ Ballot(p) \subseteq {n \in Nat : n > 0}
                          /\ \A q \in Proc \ {p} : Ballot(p) \cap Ballot(q) = {}
       /\ IsMajority(Disk)
       /\ \A S, T \in SUBSET Disk :
              IsMajority(S) /\ IsMajority(T) => (S \cup T # {})

DiskBlock == [mbal : (UNION {Ballot(p) : p \in Proc}) \cup {0},
              bal  : (UNION {Ballot(p) : p \in Proc}) \cup {0},
              inp  : Inputs \cup {NotAnInput}]

InitDB == [mbal |-> 0, bal |-> 0, inp |-> NotAnInput]

VARIABLES disk , dblock , phase, disksWritten, blocksRead

vars == <<input, output, disk , phase, dblock , disksWritten, blocksRead>>

Init == /\ input \in [Proc -> Inputs]
        /\ output = [p \in Proc |-> NotAnInput]
        /\ disk = [d \in Disk |-> [p \in Proc |-> InitDB ]]
        /\ phase = [p \in Proc |-> 0]
        /\ dblock = [p \in Proc |-> InitDB ]
        /\ disksWritten = [p \in Proc |-> {}]
        /\ blocksRead = [p \in Proc |-> [d \in Disk |-> {}]]
       
hasRead (p, d , q) == \E br \in blocksRead [p][d ] : br .proc = q

allBlocksRead(p) ==
  LET allRdBlks == UNION {blocksRead [p][d ] : d \in Disk }
  IN {br.block : br \in allRdBlks}

InitializePhase(p) ==
  /\ disksWritten' = [disksWritten EXCEPT ![p] = {}]
  /\ blocksRead' = [blocksRead EXCEPT ![p] = [d \in Disk |-> {}]]

StartBallot(p) ==
  /\ phase[p] \in {1, 2}
  /\ phase' = [phase EXCEPT ![p] = 1]
  /\ \E b \in Ballot(p) : /\ b > dblock [p].mbal
                          /\ dblock' = [dblock EXCEPT ![p].mbal = b]
  /\ InitializePhase(p)
  /\ UNCHANGED <<input, output, disk>>

Phase1or2Write(p, d) ==
  /\ phase[p] \in {1, 2}
  /\ disk' = [disk EXCEPT ![d][p] = dblock[p]]
  /\ disksWritten' = [disksWritten EXCEPT ![p] = @ \cup {d}]
  /\ UNCHANGED <<input, output, phase, dblock , blocksRead>>

Phase1or2Read (p, d, q) ==
  /\ d \in disksWritten[p]
  /\ IF disk [d ][q].mbal < dblock [p].mbal
     THEN /\ blocksRead' = 
                [blocksRead EXCEPT
                    ![p][d ] = @ \cup {[block |-> disk [d ][q], proc |-> q]}]
          /\ UNCHANGED <<input, output, disk , phase, dblock , disksWritten>>
     ELSE StartBallot(p)

EndPhase1or2(p) ==
  /\ IsMajority({d \in disksWritten[p] :
                    \A q \in Proc \ {p} : hasRead (p, d , q)})
  /\ \/ /\ phase[p] = 1
        /\ dblock' =
              [dblock EXCEPT
                ![p].bal = dblock [p].mbal,
                ![p].inp =
                    LET blocksSeen == allBlocksRead(p) \cup {dblock [p]}
                        nonInitBlks ==
                            {bs \in blocksSeen : bs.inp # NotAnInput}
                        maxBlk ==
                            CHOOSE b \in nonInitBlks :
                                \A c \in nonInitBlks : b.bal >= c.bal
                    IN IF nonInitBlks = {} THEN input[p]
                       ELSE maxBlk .inp ]
        /\ UNCHANGED output
     \/ /\ phase[p] = 2
        /\ output' = [output EXCEPT ![p] = dblock [p].inp]
        /\ UNCHANGED dblock
  /\ phase' = [phase EXCEPT ![p] = @ + 1]
  /\ InitializePhase(p)
  /\ UNCHANGED <<input, disk>>

Fail(p) ==
  /\ \E ip \in Inputs : input' = [input EXCEPT ![p] = ip]
  /\ phase' = [phase EXCEPT ![p] = 0]
  /\ dblock' = [dblock EXCEPT ![p] = InitDB]
  /\ output' = [output EXCEPT ![p] = NotAnInput]
  /\ InitializePhase(p)
  /\ UNCHANGED disk

Phase0Read(p, d) ==
  /\ phase[p] = 0
  /\ blocksRead' = [blocksRead EXCEPT
                        ![p][d ] = @ \cup {[block |-> disk [d][p], proc |-> p]}]
  /\ UNCHANGED <<input, output, disk , phase, dblock , disksWritten>>
  

EndPhase0(p) ==
  /\ phase[p] = 0
  /\ IsMajority({d \in Disk : hasRead (p, d, p)})
  /\ \E b \in Ballot(p) :
        /\ \A r \in allBlocksRead (p) : b > r.mbal
        /\ dblock' = [dblock EXCEPT
                        ![p] = [(CHOOSE r \in allBlocksRead (p) :
                                     \A s \in allBlocksRead (p) : r .bal >= s.bal)
                                  EXCEPT !.mbal = b]]
  /\ InitializePhase(p)
  /\ phase' = [phase EXCEPT ![p] = 1]
  /\ UNCHANGED <<input, output, disk>>

Next == \E p \in Proc :
          \/ StartBallot(p)
          \/ \E d \in Disk : \/ Phase0Read (p, d)
                             \/ Phase1or2Write(p, d)
                             \/ \E q \in Proc \ {p} : Phase1or2Read (p, d, q)
          \/ EndPhase1or2(p)
          \/ Fail(p)
          \/ EndPhase0(p)

DiskSynodSpec == Init /\ [][Next]_vars

THEOREM DiskSynodSpec => SynodSpec


=============================================================================
\* ModIFication History
\* Last modIFied Sat Jan 26 14:03:35 CET 2019 by tthai
\* Created Sat Jan 26 12:19:05 CET 2019 by tthai
