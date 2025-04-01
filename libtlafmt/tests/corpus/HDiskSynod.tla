----------------------------- MODULE HDiskSynod -----------------------------

EXTENDS DiskSynod
VARIABLES allInput, chosen

HInit == /\ Init
         /\ chosen = NotAnInput
         /\ allInput = {input[p] : p \in Proc}
        
HNext == /\ Next
         /\ chosen' = LET hasOutput(p) == output'[p] # NotAnInput
                      IN IF \/ chosen # NotAnInput
                            \/ \A p \in Proc : ~hasOutput(p)
                         THEN chosen
                         ELSE output'[CHOOSE p \in Proc : hasOutput(p)]
         /\ allInput' = allInput \cup {input'[p] : p \in Proc}        
        

HInv1 ==
  /\ input \in [Proc -> Inputs]
  /\ output \in [Proc -> Inputs \cup {NotAnInput}]
  /\ disk \in [Disk -> [Proc -> DiskBlock ]]
  /\ phase \in [Proc -> 0..3]
  /\ dblock \in [Proc -> DiskBlock ]
  /\ output \in [Proc -> Inputs \cup {NotAnInput}]
  /\ disksWritten \in [Proc -> SUBSET Disk ]
  /\ blocksRead \in [Proc -> [Disk -> SUBSET [block : DiskBlock , proc : Proc]]]
  /\ allInput \in SUBSET Inputs
  /\ chosen \in Inputs \cup {NotAnInput}
  
MajoritySet == {D \in SUBSET Disk : IsMajority(D)}

blocksOf(p) ==
  LET rdBy(q, d) == {br \in blocksRead[q][d] : br.proc = p}
  IN {dblock[p]} \cup {disk[d][p] : d \in Disk }
        \cup {br.block : br \in UNION{rdBy(q, d) : q \in Proc, d \in Disk}}

allBlocks == UNION {blocksOf(p) : p \in Proc}  

HInv2 ==
  /\ \A p \in Proc :
      \A bk \in blocksOf (p) : /\ bk.mbal \in Ballot(p) \cup {0}
                               /\ bk.bal \in Ballot(p) \cup {0}
                               (*/\ (bk.bal = 0) ≡ (bk.inp = NotAnInput)*)
                               /\ (bk.bal = 0) <=> (bk.inp = NotAnInput)
                               /\ bk.mbal >= bk.bal
                               /\ bk.inp \in allInput \cup {NotAnInput}
  /\ \A p \in Proc, d \in Disk :
        /\ (d \in disksWritten[p]) => /\ phase[p] \in {1, 2}
                                      /\ disk[d][p] = dblock[p]
        /\ (phase[p] \in {1, 2}) => /\ (blocksRead [p][d ] # {}) =>
                                            (d \in disksWritten[p])
                                    /\ ~hasRead (p, d, p)
  /\ \A p \in Proc :
        /\ (phase[p] = 0) => /\ dblock[p] = InitDB
                             /\ disksWritten[p] = {}
                             /\ \A d \in Disk : \A br \in blocksRead[p][d] :
                                      /\ br.proc = p
                                      /\ br.block = disk [d ][p]
        /\ (phase[p] # 0) => /\ dblock[p].mbal \in Ballot(p)
                             /\ dblock[p].bal \in Ballot(p) \cup {0}
                             /\ \A d \in Disk : \A br \in blocksRead [p][d ] :
                                          br.block.mbal < dblock[p].mbal
        /\ (phase[p] \in {2, 3}) => (dblock[p].bal = dblock[p].mbal)
        /\ output[p] = IF phase[p] = 3 THEN dblock[p].inp ELSE NotAnInput
  /\ chosen \in allInput \cup {NotAnInput}
  /\ \A p \in Proc : /\ input[p] \in allInput
                     /\ (chosen = NotAnInput) => (output[p] = NotAnInput)

HInv3 == \A p, q \in Proc, d \in Disk :
              /\ phase[p] \in {1, 2}
              /\ phase[q] \in {1, 2}
              /\ hasRead (p, d, q)
              /\ hasRead (q, d, p)
            => \/ [block |-> dblock [q], proc |-> q] \in blocksRead[p][d]
               \/ [block |-> dblock [p], proc |-> p] \in blocksRead[q][d]

HInv4 ==
  \A p \in Proc :
      /\ (phase[p] # 0) =>
            /\ \A bk \in blocksOf(p) : dblock[p].mbal >= bk.bal
            /\ \A D \in MajoritySet :
                    \E d \in D : /\ dblock[p].mbal >= disk[d][p].mbal
                                 /\ dblock[p].bal >= disk[d][p].bal
      /\ (phase[p] = 1) => (\A bk \in blocksOf(p) : dblock[p].mbal > bk.bal)
      /\ (phase[p] \in {2, 3}) =>
            (\E D \in MajoritySet : \A d \in D : disk[d][p].mbal = dblock[p].bal)
      /\ \A bk \in blocksOf(p) :
            \E D \in MajoritySet : \A d \in D : disk[d][p].mbal >= bk.bal

maxBalInp(b, v ) == \A bk \in allBlocks : (bk.bal >= b) => (bk.inp = v)

HInv5 ==
  \A p \in Proc :
    (phase[p] = 2) => \/ maxBalInp(dblock[p].bal , dblock[p].inp)
                      \/  \E D \in MajoritySet, q \in Proc :
                              \A d \in D : /\ disk[d][q].mbal > dblock [p].bal
                                           /\ ~hasRead (p, d , q)

valueChosen(v) ==
  \E b \in UNION {Ballot(p) : p \in Proc} :
      /\ maxBalInp(b, v)
      /\ \E p \in Proc, D \in MajoritySet :
              \A d \in D : /\ disk[d][p].bal >= b
                           /\ \A q \in Proc :
                                  /\ phase[q] = 1
                                  /\ dblock[q].mbal >= b
                                  /\ hasRead(q, d, p)
                                => (\E br \in blocksRead[q][d] : br.block.bal >= b)

HInv6 == /\ (chosen # NotAnInput) => valueChosen(chosen)
         /\ \A p \in Proc : output[p] \in {chosen, NotAnInput}



=============================================================================
\* Modification History
\* Last modified Sat Jan 26 15:52:41 CET 2019 by tthai
\* Created Sat Jan 26 15:23:57 CET 2019 by tthai
