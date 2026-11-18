
/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */
/* WARNING: Removing unreachable block (ram,0x000140144a43) */
/* WARNING: Removing unreachable block (ram,0x000140144a4b) */
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_140141d60(longlong *param_1)

{
  uint *puVar1;
  ushort uVar2;
  short sVar3;
  bool bVar4;
  bool bVar5;
  char cVar6;
  char cVar7;
  int iVar8;
  int iVar9;
  undefined4 uVar10;
  longlong *plVar11;
  float *pfVar12;
  undefined8 uVar13;
  float *pfVar14;
  undefined8 uVar15;
  ulonglong uVar16;
  char *pcVar17;
  ulonglong uVar18;
  longlong lVar19;
  longlong lVar20;
  undefined8 uVar21;
  ulonglong uVar22;
  ulonglong *puVar23;
  ulonglong uVar24;
  longlong lVar25;
  ulonglong uVar26;
  ulonglong uVar27;
  uint uVar28;
  float fVar29;
  float fVar30;
  float fVar31;
  float fVar32;
  float fVar33;
  float fVar34;
  float fVar35;
  float fVar36;
  float fVar37;
  float fVar38;
  float fVar39;
  undefined1 auStack_6c8 [32];
  float *local_6a8;
  float *local_6a0;
  float *local_698;
  float *local_690;
  float *local_688;
  char local_678;
  float local_670;
  float fStack_66c;
  float local_668;
  float fStack_664;
  float local_660;
  float fStack_65c;
  float local_658;
  float fStack_654;
  float local_650;
  float local_648;
  char local_644;
  float local_640 [2];
  float local_638;
  float fStack_634;
  float local_630;
  float local_628;
  char local_624;
  float local_620;
  float local_61c;
  float local_618;
  int local_614;
  char local_610;
  float local_60c;
  float local_608;
  float local_604;
  float local_600;
  float local_5fc;
  float local_5f8;
  float fStack_5f4;
  float local_5f0;
  float local_5e8;
  float local_5e4;
  float local_5e0 [2];
  undefined8 local_5d8;
  float local_5d0;
  uint local_5c8 [2];
  ulonglong local_5c0;
  float local_5b8;
  float local_5b4;
  float local_5b0;
  float local_5ac;
  float local_5a8;
  float local_5a4;
  float local_5a0;
  float local_59c;
  float local_598;
  float local_590;
  float local_58c;
  undefined4 local_588;
  int local_584;
  float local_580;
  undefined1 local_57c [4];
  ulonglong local_578;
  undefined8 local_570;
  float local_568;
  float local_560;
  float local_55c;
  float local_558;
  undefined8 local_550;
  float local_548;
  float local_540;
  float local_53c;
  float local_538;
  float local_530;
  float local_52c;
  float local_528;
  float local_520;
  float local_51c;
  float local_518;
  float local_510;
  float local_50c;
  float local_508;
  float local_500;
  float local_4fc;
  float local_4f8;
  float local_4f4;
  float local_4f0;
  float local_4ec;
  float local_4e8;
  float local_4e4;
  float local_4e0;
  float local_4d8;
  undefined8 local_4d0;
  float local_4c8;
  float local_4c0;
  float local_4bc;
  undefined4 local_4b8;
  float local_4b4;
  float local_4b0;
  float local_4a8;
  float fStack_4a4;
  float local_4a0;
  float local_498;
  float local_494;
  float local_490;
  float local_48c;
  void *local_488;
  undefined8 local_480;
  undefined8 local_478;
  undefined8 local_468;
  float local_460;
  float local_45c;
  float local_458;
  float local_450;
  float local_44c;
  float local_448;
  float local_440;
  float fStack_43c;
  float local_438;
  float fStack_434;
  float local_430;
  float fStack_42c;
  float local_428;
  float fStack_424;
  float local_420;
  longlong local_418;
  longlong local_410;
  undefined8 local_408;
  float local_400;
  float fStack_3fc;
  float local_3f8;
  float local_3f4;
  float local_3f0;
  float local_3ec;
  float local_3e8;
  float fStack_3e4;
  float local_3e0;
  undefined1 local_3d8 [32];
  undefined1 local_3b8;
  undefined7 uStack_3b7;
  undefined8 local_3a8;
  ulonglong local_3a0;
  undefined1 local_380 [36];
  int local_35c;
  undefined8 *local_358;
  undefined8 *local_348;
  char local_340;
  undefined1 local_338 [304];
  undefined1 local_208 [304];
  ulonglong local_d8;
  
  local_408 = 0xfffffffffffffffe;
  local_d8 = DAT_141a96e80 ^ (ulonglong)auStack_6c8;
  *(byte *)((longlong)param_1 + 0xb8c) = *(byte *)((longlong)param_1 + 0xb8c) | 2;
  FUN_14011ae70(param_1 + 0x12f,0);
  *(undefined4 *)(param_1 + 0x133) = 0xbf800000;
  if ((*(byte *)((longlong)param_1 + 0x1ed) & 1) != 0) {
    return;
  }
  *(byte *)((longlong)param_1 + 0xb8c) = *(byte *)((longlong)param_1 + 0xb8c) | 2;
  puVar23 = (ulonglong *)(param_1 + 0x38);
  iVar8 = FUN_1402e3020(param_1[0x2a],puVar23);
  if (iVar8 < 2) {
    iVar8 = 2;
  }
  local_584 = iVar8;
  plVar11 = (longlong *)FUN_1402c9ae0(param_1);
  if (((((*(byte *)((longlong)param_1 + 0x1ee) & 1) != 0) ||
       (cVar6 = (**(code **)(*plVar11 + 0x318))(plVar11), cVar6 != '\0')) ||
      (cVar6 = (**(code **)(*plVar11 + 800))(plVar11), cVar6 != '\0')) ||
     ((cVar6 = (**(code **)(*plVar11 + 0xaf0))(plVar11), cVar6 != '\0' ||
      (fVar29 = (float)(**(code **)(*plVar11 + 0x548))(plVar11),
      *(float *)(param_1 + 0x135) * DAT_141543478 <= fVar29)))) {
    fVar29 = DAT_1415432f4;
    pfVar14 = (float *)(param_1 + 0x18);
    local_58c = DAT_141796e3c * *(float *)((longlong)param_1 + 0xd4) +
                DAT_141796e38 * *(float *)(param_1 + 0x19) +
                DAT_141796e40 * *(float *)(param_1 + 0x1c);
    local_590 = DAT_141796e3c * *(float *)(param_1 + 0x1a) +
                DAT_141796e38 * *(float *)((longlong)param_1 + 0xc4) +
                DAT_141796e40 * *(float *)((longlong)param_1 + 0xdc);
    local_4bc = DAT_141796e3c * *(float *)((longlong)param_1 + 0xcc) + DAT_141796e38 * *pfVar14 +
                DAT_141796e40 * *(float *)(param_1 + 0x1b);
    local_638 = *(float *)(param_1 + 0x25);
    fStack_634 = *(float *)((longlong)param_1 + 300);
    local_630 = *(float *)(param_1 + 0x26);
    if ((*(byte *)((longlong)param_1 + 0xb8c) & 8) != 0) {
      lVar19 = param_1[0x141];
      local_638 = *(float *)(lVar19 + 0x238);
      fStack_634 = *(float *)(lVar19 + 0x23c);
      local_630 = *(float *)(lVar19 + 0x240);
    }
    local_670 = *pfVar14;
    fStack_66c = *(float *)((longlong)param_1 + 0xc4);
    local_668 = *(float *)(param_1 + 0x19);
    fStack_664 = *(float *)((longlong)param_1 + 0xcc);
    local_660 = *(float *)(param_1 + 0x1a);
    fStack_65c = *(float *)((longlong)param_1 + 0xd4);
    local_658 = *(float *)(param_1 + 0x1b);
    fStack_654 = *(float *)((longlong)param_1 + 0xdc);
    local_650 = *(float *)(param_1 + 0x1c);
    local_500 = DAT_1415432f4;
    local_4fc = 0.0;
    local_4f8 = 0.0;
    local_4f4 = 0.0;
    local_4f0 = DAT_1415432f4;
    local_4ec = 0.0;
    local_4e8 = 0.0;
    local_4e4 = 0.0;
    local_4e0 = DAT_1415432f4;
    local_640[0] = 0.0;
    fVar30 = (float)FUN_140102240(param_1,&local_670,local_640);
    *(float *)((longlong)param_1 + 0x99c) = fVar30;
    local_500 = local_670;
    local_4fc = fStack_66c;
    local_4f8 = local_668;
    local_4f4 = fStack_664;
    local_4f0 = local_660;
    local_4ec = fStack_65c;
    local_4e8 = local_658;
    local_4e4 = fStack_654;
    local_4e0 = local_650;
    *(float *)(param_1 + 0x134) = fVar30;
    local_5a8 = (float)iVar8 - fVar29;
    fVar39 = fVar30 / local_5a8;
    local_618 = fVar39;
    if (0.0 < fVar30) {
      fVar30 = (float)FUN_1402e3050(param_1[0x2a] + 0x488,puVar23);
      if (((0.0 < fVar30) && (iVar9 = FUN_1402e3020(param_1[0x2a] + 0x60,puVar23), iVar9 != 3)) &&
         ((*(float *)(param_1 + 0x135) <= 0.0 ||
          (fVar30 < *(float *)((longlong)param_1 + 0x99c) / *(float *)(param_1 + 0x135))))) {
        FUN_1402c9b20(param_1,"1Branch removed (length/radius larger than kill ratio)");
        *(byte *)((longlong)param_1 + 0x1ec) = *(byte *)((longlong)param_1 + 0x1ec) & 0xfd;
        FUN_1401349c0(param_1);
        FUN_14011ae70(param_1 + 0x12f,0);
        return;
      }
      fVar30 = (float)FUN_1402e3050(param_1[0x2a] + 0x480,puVar23);
      fVar30 = fVar30 * *(float *)(param_1 + 0x135) * DAT_14102c9a0;
      if (fVar30 <= 0.0) {
        *(undefined4 *)((longlong)param_1 + 0xab4) = 0;
      }
      else {
        local_640[0] = (*(float *)((longlong)param_1 + 0x99c) - fVar30) / (fVar30 * DAT_1415434c8);
        local_5e0[0] = 0.0;
        local_600 = fVar29;
        if (local_640[0] <= 0.0) {
          pfVar12 = local_5e0;
        }
        else {
          pfVar12 = local_640;
          if (fVar29 <= local_640[0]) {
            pfVar12 = &local_600;
          }
        }
        *(float *)((longlong)param_1 + 0xab4) = fVar29 - *pfVar12;
      }
      fVar30 = *(float *)(param_1 + 0x135);
      local_640[0] = (float)FUN_1400fd260(param_1);
      local_640[0] = local_640[0] * fVar30;
      local_600 = 0.0;
      pfVar12 = local_640;
      if (local_640[0] <= 0.0) {
        pfVar12 = &local_600;
      }
      local_600 = *pfVar12;
      if ((*(byte *)((longlong)param_1 + 0xb8c) & 0x20) != 0) {
        local_600 = fVar30;
      }
      fVar33 = fStack_65c * 0.0 + local_668 * 0.0 + local_650;
      fVar32 = local_660 * 0.0 + fStack_66c * 0.0 + fStack_654;
      fVar34 = local_670 * 0.0 + fStack_664 * 0.0 + local_658;
      local_570 = CONCAT44(fVar32,fVar34);
      local_568 = fVar33;
      fVar30 = (float)FUN_1402e3050(param_1[0x2a] + 0x3b0,puVar23);
      if (((((0.0 < fVar30) && ((*(byte *)((longlong)param_1 + 0x1ee) & 1) == 0)) &&
           (cVar6 = FUN_140109fb0(param_1), cVar6 == '\0')) ||
          (((fVar31 = (float)FUN_1402e3050(param_1[0x2a] + 0x3b8,puVar23), fVar30 = DAT_1415433a0,
            0.0 < fVar31 && ((*(byte *)((longlong)param_1 + 0x1ee) & 1) == 0)) &&
           ((fVar32 * (float)((uint)DAT_141796e3c ^ DAT_141543430) +
             fVar34 * (float)((uint)DAT_141796e38 ^ DAT_141543430) +
             fVar33 * (float)((uint)DAT_141796e40 ^ DAT_141543430) + fVar29) * DAT_1415433a0 <
            fVar31)))) ||
         ((((fVar31 = (float)FUN_1402e3050(param_1[0x2a] + 0x3c0,puVar23), 0.0 < fVar31 &&
            ((*(byte *)((longlong)param_1 + 0x1ee) & 1) == 0)) &&
           ((fVar32 * DAT_141796e3c + fVar34 * DAT_141796e38 + fVar33 * DAT_141796e40 + fVar29) *
            fVar30 < fVar31)) ||
          (fVar32 = (float)FUN_1402e3050(param_1[0x2a] + 0x508,puVar23), fVar30 < fVar32)))) {
        *(byte *)((longlong)param_1 + 0x1ec) = *(byte *)((longlong)param_1 + 0x1ec) & 0xfd;
        return;
      }
      local_610 = FUN_1402e3080(param_1[0x2a] + 0x408,puVar23);
      local_494 = (float)FUN_1402e3050(param_1[0x2a] + 0x2e8,puVar23);
      local_494 = local_494 * DAT_1415434b0;
      uVar10 = FUN_1402e3020(param_1[0x2a] + 800,puVar23);
      FUN_1403116c0(local_57c,uVar10);
      FUN_140310c90(local_338,local_57c);
      FUN_140310c90(local_208,local_57c);
      local_4b8 = FUN_1402e3050(param_1[0x2a] + 0x2f0,puVar23);
      local_48c = (float)FUN_1402e3050(param_1[0x2a] + 0x2f8,puVar23);
      local_48c = local_48c * DAT_141041ae4;
      fVar32 = (float)FUN_1402e3050(param_1[0x2a] + 0x530,puVar23);
      fVar30 = DAT_14102c870;
      fVar32 = fVar32 * DAT_14102c870;
      local_580 = 0.0;
      local_588 = 0;
      local_5b8 = 0.0;
      local_5b0 = fVar32;
      if (fVar32 == 0.0) goto LAB_140142685;
      uVar13 = FUN_14023a620(param_1[0x28],0xa6);
      uVar2 = *(ushort *)(param_1[0x2a] + 0x534);
      uVar18 = (ulonglong)uVar2;
      if (uVar2 == 0xffff) {
LAB_140142598:
        uVar21 = 0;
        uVar15 = uVar13;
      }
      else {
        uVar18 = *puVar23;
        if (*(longlong *)(uVar18 + 8 + (longlong)(short)uVar2 * 0x18) == 0) goto LAB_140142598;
        uVar15 = *(undefined8 *)(uVar18 + 8 + (longlong)(short)uVar2 * 0x18);
        uVar21 = uVar13;
      }
      FUN_140087a60(uVar15,uVar18,uVar21);
      uVar13 = FUN_14023a620(param_1[0x28],0xa6);
      uVar2 = *(ushort *)(param_1[0x2a] + 0x534);
      uVar18 = (ulonglong)uVar2;
      if (uVar2 == 0xffff) {
LAB_1401425e9:
        uVar21 = 0;
        uVar15 = uVar13;
      }
      else {
        uVar18 = *puVar23;
        if (*(longlong *)(uVar18 + 8 + (longlong)(short)uVar2 * 0x18) == 0) goto LAB_1401425e9;
        uVar15 = *(undefined8 *)(uVar18 + 8 + (longlong)(short)uVar2 * 0x18);
        uVar21 = uVar13;
      }
      FUN_140087a60(uVar15,uVar18,uVar21);
      fVar33 = (float)FUN_1402e3050(param_1[0x2a] + 0x538,puVar23);
      local_580 = fVar33;
      local_588 = FUN_1402e3050(param_1[0x2a] + 0x540,puVar23);
      local_5b8 = (float)FUN_1402e3050(param_1[0x2a] + 0x548,puVar23);
      local_5b8 = local_5b8 * fVar30;
      if (0.0 < fVar33) {
        fVar33 = fVar33 / *(float *)((longlong)param_1 + 0x99c);
        local_5b0 = fVar32 * fVar33;
        local_5b8 = local_5b8 * fVar33;
      }
      else {
        local_5b0 = 0.0;
      }
LAB_140142685:
      local_5e0[0] = (float)FUN_1402e3050(param_1[0x2a] + 0x328,puVar23);
      local_4b0 = (float)FUN_1402e3050(param_1[0x2a] + 0x410,puVar23);
      uVar18 = (param_1[0x22] - param_1[0x21]) / 0x28;
      local_5a0 = 0.0;
      local_59c = 0.0;
      local_598 = 0.0;
      uVar26 = uVar18 & 0xffffffff;
      local_640[0] = 0.0;
      local_488 = (void *)0x0;
      local_480 = 0;
      local_478 = 0;
      local_5c0 = uVar18;
      local_578 = uVar26;
      FUN_14003aec0(&local_488,uVar26 + 0x1f >> 5,local_640);
      local_468 = 0;
      FUN_14003aab0(&local_488,uVar18 & 0xffffffff);
      local_458 = DAT_141796e3c * *(float *)((longlong)param_1 + 0xd4) +
                  DAT_141796e38 * *(float *)(param_1 + 0x19) +
                  DAT_141796e40 * *(float *)(param_1 + 0x1c);
      local_45c = DAT_141796e3c * *(float *)(param_1 + 0x1a) +
                  DAT_141796e38 * *(float *)((longlong)param_1 + 0xc4) +
                  DAT_141796e40 * *(float *)((longlong)param_1 + 0xdc);
      local_460 = DAT_141796e3c * *(float *)((longlong)param_1 + 0xcc) + DAT_141796e38 * *pfVar14 +
                  DAT_141796e40 * *(float *)(param_1 + 0x1b);
      local_448 = fStack_65c * DAT_141796e3c + local_668 * DAT_141796e38 + local_650 * DAT_141796e40
      ;
      local_44c = local_660 * DAT_141796e3c + fStack_66c * DAT_141796e38 +
                  fStack_654 * DAT_141796e40;
      local_450 = local_670 * DAT_141796e38 + fStack_664 * DAT_141796e3c + local_658 * DAT_141796e40
      ;
      FUN_1400c7b20(local_3d8,*(longlong *)(param_1[0x28] + 200) + 0x5e60);
      fVar33 = (float)FUN_1402e3050(param_1[0x2a] + 0x58,puVar23);
      fVar33 = fVar33 * DAT_14102cfc8;
      fVar30 = 0.0;
      local_5e4 = 0.0;
      fVar32 = local_5e4;
      local_5b4 = fVar33;
      if ((*(byte *)((longlong)param_1 + 0x1ee) & 1) != 0) {
        plVar11 = (longlong *)FUN_1402c9ae0(param_1);
        cVar6 = (**(code **)(*plVar11 + 0x508))(plVar11);
        fVar32 = local_5e4;
        if (cVar6 == '\0') {
          plVar11 = (longlong *)FUN_1402c9ae0(param_1);
          fVar32 = local_5e4;
          while ((plVar11 != (longlong *)0x0 &&
                 (cVar6 = (**(code **)(*plVar11 + 0x310))(plVar11), fVar32 = fVar30, cVar6 == '\0'))
                ) {
            fVar32 = (float)(**(code **)(*plVar11 + 0x548))(plVar11);
            fVar30 = fVar30 + fVar32;
            plVar11 = (longlong *)FUN_1402c9ae0(plVar11);
            fVar32 = fVar30;
          }
        }
      }
      local_5e4 = fVar32;
      lVar19 = param_1[0x170];
      if (lVar19 != 0) {
        *(float *)(lVar19 + 0x28) = local_670;
        *(float *)(lVar19 + 0x2c) = fStack_66c;
        *(float *)(lVar19 + 0x30) = local_668;
        *(float *)(lVar19 + 0x34) = fStack_664;
        *(float *)(lVar19 + 0x38) = local_660;
        *(float *)(lVar19 + 0x3c) = fStack_65c;
        *(float *)(lVar19 + 0x40) = local_658;
        *(float *)(lVar19 + 0x44) = fStack_654;
        *(float *)(lVar19 + 0x48) = local_650;
        lVar19 = param_1[0x170];
        *(float *)(lVar19 + 0x4c) = *pfVar14;
        *(undefined4 *)(lVar19 + 0x50) = *(undefined4 *)((longlong)param_1 + 0xc4);
        *(int *)(lVar19 + 0x54) = (int)param_1[0x19];
        *(undefined4 *)(lVar19 + 0x58) = *(undefined4 *)((longlong)param_1 + 0xcc);
        *(int *)(lVar19 + 0x5c) = (int)param_1[0x1a];
        *(undefined4 *)(lVar19 + 0x60) = *(undefined4 *)((longlong)param_1 + 0xd4);
        *(int *)(lVar19 + 100) = (int)param_1[0x1b];
        *(undefined4 *)(lVar19 + 0x68) = *(undefined4 *)((longlong)param_1 + 0xdc);
        *(int *)(lVar19 + 0x6c) = (int)param_1[0x1c];
        fVar37 = DAT_141796e3c * *(float *)((longlong)param_1 + 0xd4) +
                 DAT_141796e38 * *(float *)(param_1 + 0x19) +
                 DAT_141796e40 * *(float *)(param_1 + 0x1c);
        fVar38 = DAT_141796e3c * *(float *)(param_1 + 0x1a) +
                 DAT_141796e38 * *(float *)((longlong)param_1 + 0xc4) +
                 DAT_141796e40 * *(float *)((longlong)param_1 + 0xdc);
        fVar35 = DAT_141796e3c * *(float *)((longlong)param_1 + 0xcc) + DAT_141796e38 * *pfVar14 +
                 DAT_141796e40 * *(float *)(param_1 + 0x1b);
        fVar31 = fStack_65c * DAT_141796e3c + local_668 * DAT_141796e38 + local_650 * DAT_141796e40;
        fVar36 = local_660 * DAT_141796e3c + fStack_66c * DAT_141796e38 + fStack_654 * DAT_141796e40
        ;
        fVar34 = local_670 * DAT_141796e38 + fStack_664 * DAT_141796e3c + local_658 * DAT_141796e40;
        lVar19 = param_1[0x170];
        *(undefined8 *)(lVar19 + 0x70) = 0x3f800000;
        *(undefined8 *)(lVar19 + 0x78) = 0;
        *(undefined8 *)(lVar19 + 0x80) = 0x3f800000;
        *(undefined8 *)(lVar19 + 0x88) = 0;
        *(undefined4 *)(lVar19 + 0x90) = 0x3f800000;
        fVar30 = sqrtf(fVar35 * fVar35 + fVar38 * fVar38 + fVar37 * fVar37);
        fVar32 = sqrtf(fVar34 * fVar34 + fVar36 * fVar36 + fVar31 * fVar31);
        if (fVar32 * fVar30 == 0.0) {
          fVar32 = 0.0;
        }
        else {
          fVar30 = (fVar34 * fVar35 + fVar36 * fVar38 + fVar31 * fVar37) / (fVar32 * fVar30);
          if (fVar30 < fVar29) {
            fVar32 = DAT_14102c870;
            if (DAT_14102c874 < fVar30) {
              fVar30 = acosf(fVar30);
              fVar32 = fVar30 * DAT_14102c808;
            }
          }
          else {
            fVar32 = 0.0;
          }
        }
        fVar30 = local_5e4;
        fVar33 = local_5b4;
        if (DAT_14102cfd0 < fVar32) {
          fVar33 = fVar35 * fVar36 - fVar34 * fVar38;
          fVar34 = fVar34 * fVar37 - fVar35 * fVar31;
          fVar31 = fVar31 * fVar38 - fVar36 * fVar37;
          local_5d8 = CONCAT44(fVar34,fVar31);
          fVar30 = fVar31 * fVar31 + fVar34 * fVar34 + fVar33 * fVar33;
          local_640[0] = fVar32;
          local_5d0 = fVar33;
          if (fVar30 != 0.0) {
            local_5d0 = sqrtf(fVar30);
            local_5d0 = fVar29 / local_5d0;
            local_5d8 = CONCAT44(fVar34 * local_5d0,fVar31 * local_5d0);
            local_5d0 = fVar33 * local_5d0;
          }
          FUN_140593bf0(param_1[0x170] + 0x70,&local_5d8,local_640);
          fVar30 = local_5e4;
          fVar33 = local_5b4;
        }
      }
      local_644 = '\0';
      local_640[0] = -NAN;
      FUN_14011ae70(param_1 + 0x12f,(longlong)iVar8);
      local_570._0_4_ = local_638;
      local_570._4_4_ = fStack_634;
      local_568 = local_630;
      local_440 = local_670;
      fStack_43c = fStack_66c;
      local_438 = local_668;
      fStack_434 = fStack_664;
      local_430 = local_660;
      fStack_42c = fStack_65c;
      local_428 = local_658;
      fStack_424 = fStack_654;
      local_420 = local_650;
      local_678 = '\0';
      local_5fc = 0.0;
      local_5a4 = 0.0;
      local_614 = 0;
      local_418 = 0;
      if (0 < iVar8) {
        do {
          iVar8 = local_614;
          cVar6 = local_678;
          local_5ac = (float)local_614;
          local_604 = local_5ac / local_5a8;
          lVar25 = local_418 * 0xac;
          lVar19 = param_1[0x12f];
          *(float *)(lVar25 + 0x54 + lVar19) = local_638;
          *(float *)(lVar25 + 0x58 + lVar19) = fStack_634;
          *(float *)(lVar25 + 0x5c + lVar19) = local_630;
          local_410 = lVar25;
          if ((local_610 != '\0') && (local_630 <= 0.0)) {
            if (((*(byte *)((longlong)param_1 + 0x1ee) & 1) != 0) && (local_644 == '\0')) {
              FUN_1402c9b20(param_1,"1Ground pruning ignored on branch extension.");
            }
            local_644 = '\x01';
          }
          local_5ac = local_5ac * fVar39;
          local_608 = (local_5ac + fVar30) * fVar33;
          FUN_140594750(&local_670,&local_608);
          fVar34 = fStack_65c * DAT_141796e3c + local_668 * DAT_141796e38 +
                   local_650 * DAT_141796e40;
          fVar33 = local_660 * DAT_141796e3c + fStack_66c * DAT_141796e38 +
                   fStack_654 * DAT_141796e40;
          fVar32 = local_670 * DAT_141796e38 + fStack_664 * DAT_141796e3c +
                   local_658 * DAT_141796e40;
          uVar13 = CONCAT44(fVar33,fVar32);
          fVar30 = local_670;
          local_4d0 = uVar13;
          local_4c8 = fVar34;
          local_4a8 = fVar32;
          fStack_4a4 = fVar33;
          local_4a0 = fVar34;
          if (((int)local_5c0 != 0) && (cVar6 == '\0')) {
            if (local_35c < 2) {
              if ((int)local_5c0 != 0) {
                lVar19 = 0;
                lVar20 = 0;
                uVar18 = local_578;
                do {
                  cVar6 = FUN_1402ca0c0(lVar20 + param_1[0x21],param_1,0);
                  if (cVar6 != '\0') {
                    FUN_1402ca160(lVar20 + param_1[0x21]);
                    local_688 = &local_5a0;
                    local_690 = &local_460;
                    local_698 = &local_450;
                    local_6a0 = &local_670;
                    local_6a8 = &local_638;
                    FUN_1401f1330(*(undefined8 *)(param_1[0x21] + 0x10 + lVar19));
                    fVar32 = fVar32 + local_5a0;
                    fVar33 = fVar33 + local_59c;
                    fVar34 = fVar34 + local_598;
                  }
                  lVar20 = lVar20 + 0x28;
                  lVar19 = lVar19 + 0x28;
                  uVar18 = uVar18 - 1;
                  fVar29 = DAT_1415432f4;
                } while (uVar18 != 0);
              }
            }
            else {
              FUN_1403116c0(local_5c8,0x1bc);
              local_61c = 0.0;
              local_620 = 0.0;
              local_608 = 0.0;
              iVar8 = 0;
              if (0 < local_35c) {
                do {
                  uVar18 = 0;
                  fStack_3fc = 0.0;
                  local_3f8 = 0.0;
                  local_3f4 = 0.0;
                  local_3ec = 0.0;
                  local_3e8 = 0.0;
                  fStack_3e4 = 0.0;
                  local_5c8[0] = local_5c8[0] + 1;
                  if (9999 < local_5c8[0]) {
                    local_5c8[0] = 0;
                  }
                  local_4c0 = *(float *)(PTR_DAT_1417901e0 + (ulonglong)local_5c8[0] * 4) *
                              _DAT_1410458d4 - _DAT_1410458c4;
                  local_550 = uVar13;
                  local_548 = fVar34;
                  local_400 = fVar29;
                  local_3f0 = fVar29;
                  local_3e0 = fVar29;
                  FUN_1405942b0(&local_400,&local_4c0);
                  uVar26 = (ulonglong)(local_5c8[0] + 1);
                  if (9999 < local_5c8[0] + 1) {
                    uVar26 = uVar18;
                  }
                  local_5c8[0] = (uint)uVar26;
                  local_4b4 = *(float *)(PTR_DAT_1417901e0 + uVar26 * 4) * _DAT_1410458d4 -
                              _DAT_1410458c4;
                  FUN_1405944f0(&local_400,&local_4b4);
                  local_60c = local_670;
                  local_5e8 = fStack_3fc * fStack_664 + local_670 * local_400 +
                              local_3f8 * local_658;
                  local_628 = fStack_3fc * local_660 + fStack_66c * local_400 +
                              local_3f8 * fStack_654;
                  local_648 = fStack_3fc * fStack_65c + local_668 * local_400 +
                              local_3f8 * local_650;
                  fVar35 = local_670 * local_3f4 + local_3f0 * fStack_664 + local_3ec * local_658;
                  fVar31 = local_3f0 * local_660 + local_3f4 * fStack_66c + local_3ec * fStack_654;
                  local_3ec = local_3f0 * fStack_65c + local_3f4 * local_668 + local_3ec * local_650
                  ;
                  fVar33 = local_670 * local_3e8 + fStack_3e4 * fStack_664 + local_3e0 * local_658;
                  fVar32 = fStack_3e4 * local_660 + local_3e8 * fStack_66c + local_3e0 * fStack_654;
                  local_3e0 = fStack_3e4 * fStack_65c + local_3e8 * local_668 +
                              local_3e0 * local_650;
                  uVar26 = uVar18;
                  uVar22 = local_578;
                  fVar29 = (float)local_550;
                  fVar39 = local_548;
                  fVar30 = local_550._4_4_;
                  local_400 = local_5e8;
                  fStack_3fc = local_628;
                  local_3f8 = local_648;
                  local_3f4 = fVar35;
                  local_3f0 = fVar31;
                  local_3e8 = fVar33;
                  fStack_3e4 = fVar32;
                  if ((uint)local_5c0 != 0) {
                    do {
                      cVar6 = FUN_1402ca0c0(uVar26 + param_1[0x21],param_1,0);
                      if (cVar6 != '\0') {
                        FUN_1402ca160(uVar26 + param_1[0x21]);
                        local_688 = &local_5a0;
                        local_690 = &local_460;
                        local_698 = &local_450;
                        local_6a0 = &local_400;
                        local_6a8 = &local_638;
                        FUN_1401f1330(*(undefined8 *)(param_1[0x21] + 0x10 + uVar18));
                        fVar29 = fVar29 + local_5a0;
                        fVar30 = fVar30 + local_59c;
                        fVar39 = fVar39 + local_598;
                      }
                      uVar18 = uVar18 + 0x28;
                      uVar22 = uVar22 - 1;
                      uVar26 = uVar26 + 0x28;
                    } while (uVar22 != 0);
                    uVar13 = CONCAT44(fStack_4a4,local_4a8);
                  }
                  local_61c = local_61c + fVar29;
                  local_620 = local_620 + fVar30;
                  local_608 = local_608 + fVar39;
                  iVar8 = iVar8 + 1;
                  fVar29 = DAT_1415432f4;
                } while (iVar8 < local_35c);
              }
              fVar32 = fVar29 / (float)local_35c;
              fVar34 = local_608 * fVar32;
              fVar33 = local_620 * fVar32;
              fVar32 = fVar32 * local_61c;
              lVar25 = local_410;
            }
            fVar30 = local_670;
            fVar39 = fVar32 * fVar32 + fVar33 * fVar33 + fVar34 * fVar34;
            if (fVar39 != 0.0) {
              fVar39 = sqrtf(fVar39);
              fVar39 = fVar29 / fVar39;
              fVar32 = fVar32 * fVar39;
              fVar33 = fVar33 * fVar39;
              fVar34 = fVar34 * fVar39;
            }
            fVar36 = local_4c8;
            fVar31 = (float)local_4d0;
            fVar35 = local_4d0._4_4_;
            fVar39 = sqrtf((float)local_4d0 * (float)local_4d0 + local_4d0._4_4_ * local_4d0._4_4_ +
                           local_4c8 * local_4c8);
            fVar37 = sqrtf(fVar32 * fVar32 + fVar33 * fVar33 + fVar34 * fVar34);
            if (fVar37 * fVar39 == 0.0) {
              fVar37 = 0.0;
            }
            else {
              fVar39 = (fVar31 * fVar32 + fVar35 * fVar33 + fVar36 * fVar34) / (fVar37 * fVar39);
              if (fVar39 < fVar29) {
                fVar37 = DAT_14102c870;
                if (DAT_14102c874 < fVar39) {
                  fVar39 = acosf(fVar39);
                  fVar37 = fVar39 * DAT_14102c808;
                }
              }
              else {
                fVar37 = 0.0;
              }
            }
            fVar39 = local_618;
            iVar8 = local_614;
            local_4d8 = fVar37;
            cVar6 = local_678;
            if (DAT_1410458b8 < fVar37) {
              fVar30 = fVar33 * fVar31 - fVar35 * fVar32;
              fVar31 = fVar36 * fVar32 - fVar34 * fVar31;
              fVar32 = fVar35 * fVar34 - fVar36 * fVar33;
              fVar39 = fVar32 * fVar32 + fVar31 * fVar31 + fVar30 * fVar30;
              local_510 = fVar32;
              local_50c = fVar31;
              local_508 = fVar30;
              if (fVar39 != 0.0) {
                local_508 = sqrtf(fVar39);
                local_508 = fVar29 / local_508;
                local_510 = fVar32 * local_508;
                local_50c = fVar31 * local_508;
                local_508 = fVar30 * local_508;
              }
              fStack_3fc = 0.0;
              local_3f8 = 0.0;
              local_3f4 = 0.0;
              local_3ec = 0.0;
              local_3e8 = 0.0;
              fStack_3e4 = 0.0;
              local_400 = fVar29;
              local_3f0 = fVar29;
              local_3e0 = fVar29;
              FUN_140593bf0(&local_400,&local_510);
              local_648 = local_670 * local_400 + local_3f4 * fStack_66c + local_3e8 * local_668;
              local_628 = local_670 * fStack_3fc + local_3f0 * fStack_66c + fStack_3e4 * local_668;
              local_668 = local_670 * local_3f8 + local_3ec * fStack_66c + local_3e0 * local_668;
              fVar29 = local_3e8 * fStack_65c;
              fVar32 = fStack_3fc * fStack_664;
              fVar39 = fStack_3e4 * fStack_65c;
              fStack_65c = local_3f8 * fStack_664 + local_3ec * local_660 + local_3e0 * fStack_65c;
              fVar34 = local_3e8 * local_650;
              fVar30 = fStack_3fc * local_658;
              fVar33 = fStack_3e4 * local_650;
              local_650 = local_3f8 * local_658 + local_3ec * fStack_654 + local_3e0 * local_650;
              local_670 = local_648;
              fStack_66c = local_628;
              fStack_664 = fStack_664 * local_400 + local_3f4 * local_660 + fVar29;
              local_660 = fVar32 + local_3f0 * local_660 + fVar39;
              local_658 = local_658 * local_400 + local_3f4 * fStack_654 + fVar34;
              fStack_654 = fVar30 + local_3f0 * fStack_654 + fVar33;
              local_5e8 = local_668;
              FUN_140594980(&local_670);
              fVar30 = local_670;
              fVar39 = local_618;
              fVar29 = DAT_1415432f4;
              iVar8 = local_614;
              cVar6 = local_678;
            }
          }
          fVar32 = local_5b0;
          puVar23 = (ulonglong *)(param_1 + 0x38);
          pfVar14 = (float *)(lVar25 + param_1[0x12f]);
          *pfVar14 = fVar30;
          pfVar14[1] = fStack_66c;
          pfVar14[2] = local_668;
          pfVar14[3] = fStack_664;
          pfVar14[4] = local_660;
          pfVar14[5] = fStack_65c;
          pfVar14[6] = local_658;
          pfVar14[7] = fStack_654;
          pfVar14[8] = local_650;
          if ((local_5b0 != 0.0) && (1 < iVar8)) {
            uVar13 = FUN_14023a620(param_1[0x28],0xa6);
            sVar3 = *(short *)(param_1[0x2a] + 0x534);
            if (sVar3 != -1) {
              if (*(longlong *)(*puVar23 + 8 + (longlong)sVar3 * 0x18) != 0) {
                uVar13 = *(undefined8 *)(*puVar23 + 8 + (longlong)sVar3 * 0x18);
              }
            }
            fVar30 = (float)FUN_1401020d0(uVar13);
            uVar13 = FUN_14023a620(param_1[0x28],0xa6);
            sVar3 = *(short *)(param_1[0x2a] + 0x534);
            if (sVar3 != -1) {
              if (*(longlong *)(*puVar23 + 8 + (longlong)sVar3 * 0x18) != 0) {
                uVar13 = *(undefined8 *)(*puVar23 + 8 + (longlong)sVar3 * 0x18);
              }
            }
            fVar33 = (float)FUN_1401020d0(uVar13);
            uVar13 = FUN_14023a620(param_1[0x28],0xa6);
            sVar3 = *(short *)(param_1[0x2a] + 0x534);
            if (sVar3 != -1) {
              if (*(longlong *)(*puVar23 + 8 + (longlong)sVar3 * 0x18) != 0) {
                uVar13 = *(undefined8 *)(*puVar23 + 8 + (longlong)sVar3 * 0x18);
              }
            }
            fVar34 = (float)FUN_1401020d0(uVar13);
            local_648 = ((fVar34 - fVar33) - (fVar33 - fVar30)) * fVar32;
            FUN_1405944f0(&local_670,&local_648);
            fVar30 = local_5b8;
            if (local_5b8 != 0.0) {
              uVar13 = FUN_14023a620(param_1[0x28],0xa9);
              sVar3 = *(short *)(param_1[0x2a] + 0x54c);
              if (sVar3 != -1) {
                if (*(longlong *)(*puVar23 + 8 + (longlong)sVar3 * 0x18) != 0) {
                  uVar13 = *(undefined8 *)(*puVar23 + 8 + (longlong)sVar3 * 0x18);
                }
              }
              fVar32 = (float)FUN_1401020d0(uVar13);
              uVar13 = FUN_14023a620(param_1[0x28],0xa9);
              sVar3 = *(short *)(param_1[0x2a] + 0x54c);
              if (sVar3 != -1) {
                if (*(longlong *)(*puVar23 + 8 + (longlong)sVar3 * 0x18) != 0) {
                  uVar13 = *(undefined8 *)(*puVar23 + 8 + (longlong)sVar3 * 0x18);
                }
              }
              fVar33 = (float)FUN_1401020d0(uVar13);
              uVar13 = FUN_14023a620(param_1[0x28],0xa9);
              sVar3 = *(short *)(param_1[0x2a] + 0x54c);
              if (sVar3 != -1) {
                if (*(longlong *)(*puVar23 + 8 + (longlong)sVar3 * 0x18) != 0) {
                  uVar13 = *(undefined8 *)(*puVar23 + 8 + (longlong)sVar3 * 0x18);
                }
              }
              fVar34 = (float)FUN_1401020d0(uVar13);
              local_648 = ((fVar34 - fVar33) - (fVar33 - fVar32)) * fVar30;
              FUN_1405942b0(&local_670,&local_648);
            }
          }
          uVar13 = FUN_14023a620(param_1[0x28],0x5d);
          sVar3 = *(short *)(param_1[0x2a] + 0x2ec);
          if (sVar3 != -1) {
            if (*(longlong *)(*puVar23 + 8 + (longlong)sVar3 * 0x18) != 0) {
              uVar13 = *(undefined8 *)(*puVar23 + 8 + (longlong)sVar3 * 0x18);
            }
          }
          fVar30 = (float)FUN_140087a60(uVar13);
          fVar32 = (float)FUN_140311520(local_338);
          local_5fc = local_5fc + fVar32 * local_494 * fVar30;
          fVar32 = (float)FUN_140311520();
          local_5a4 = fVar32 * local_494 * fVar30;
          if (local_48c <
              (float)(double)((ulonglong)(double)local_5fc & DAT_1415434a0) +
              (float)(double)((ulonglong)(double)local_5a4 & DAT_1415434a0)) {
            FUN_1405944f0(&local_670,&local_5fc);
            FUN_1405942b0(&local_670,&local_5a4);
            local_5a4 = 0.0;
            local_5fc = 0.0;
          }
          fVar33 = local_650;
          fVar32 = fStack_654;
          fVar30 = local_658;
          local_5f8 = local_658;
          fStack_5f4 = fStack_654;
          local_5f0 = local_650;
          local_61c = local_650 * fVar39 + local_630;
          local_620 = fStack_654 * fVar39 + fStack_634;
          local_60c = local_638 + local_658 * fVar39;
          if (local_5e0[0] != 0.0) {
            fVar35 = sqrtf(local_658 * local_658 + fStack_654 * fStack_654 + local_650 * local_650);
            fVar31 = local_4bc;
            fVar34 = local_58c;
            fVar39 = local_590;
            fVar36 = sqrtf(local_4bc * local_4bc + local_590 * local_590 + local_58c * local_58c);
            if ((fVar36 * fVar35 != 0.0) &&
               (fVar39 = (fVar31 * fVar30 + fVar39 * fVar32 + fVar34 * fVar33) / (fVar36 * fVar35),
               fVar39 < fVar29)) {
              fVar34 = DAT_14102c870;
              if (DAT_14102c874 < fVar39) {
                fVar39 = acosf(fVar39);
                fVar34 = fVar39 * DAT_14102c808;
                if (fVar39 * DAT_14102c808 <= DAT_1410458b8) goto LAB_140144258;
              }
              fVar35 = local_590 * fVar30 - fVar31 * fVar32;
              fVar30 = fVar31 * fVar33 - local_58c * fVar30;
              fVar32 = local_58c * fVar32 - local_590 * fVar33;
              fVar39 = fVar32 * fVar32 + fVar30 * fVar30 + fVar35 * fVar35;
              local_520 = fVar32;
              local_51c = fVar30;
              local_518 = fVar35;
              if (fVar39 != 0.0) {
                local_518 = sqrtf(fVar39);
                local_518 = fVar29 / local_518;
                local_520 = fVar32 * local_518;
                local_51c = fVar30 * local_518;
                local_518 = local_518 * fVar35;
              }
              fStack_3fc = 0.0;
              local_3f8 = 0.0;
              local_3f4 = 0.0;
              local_3ec = 0.0;
              local_3e8 = 0.0;
              fStack_3e4 = 0.0;
              local_400 = fVar29;
              local_3f0 = fVar29;
              local_3e0 = fVar29;
              uVar13 = FUN_14023a620(param_1[0x28],0x65);
              uVar2 = *(ushort *)(param_1[0x2a] + 0x32c);
              uVar18 = (ulonglong)uVar2;
              if (uVar2 == 0xffff) {
LAB_140143fc3:
                uVar21 = 0;
                uVar15 = uVar13;
              }
              else {
                uVar18 = *puVar23;
                if (*(longlong *)(uVar18 + 8 + (longlong)(short)uVar2 * 0x18) == 0)
                goto LAB_140143fc3;
                uVar15 = *(undefined8 *)(uVar18 + 8 + (longlong)(short)uVar2 * 0x18);
                uVar21 = uVar13;
              }
              local_498 = (float)FUN_140087a60(uVar15,uVar18,uVar21);
              local_498 = fVar34 * local_5e0[0] * local_498;
              FUN_140593bf0(&local_400,&local_520);
              local_648 = local_3f4 * fStack_66c + local_670 * local_400 + local_3e8 * local_668;
              fVar35 = local_670 * fStack_3fc;
              fVar39 = fStack_3e4 * local_668;
              local_668 = local_670 * local_3f8 + local_3ec * fStack_66c + local_3e0 * local_668;
              fVar30 = local_3e8 * fStack_65c;
              fVar33 = fStack_3fc * fStack_664;
              fVar32 = fStack_3e4 * fStack_65c;
              fStack_65c = local_3f8 * fStack_664 + local_3ec * local_660 + local_3e0 * fStack_65c;
              fVar31 = local_3e8 * local_650;
              fVar29 = fStack_3fc * local_658;
              fVar34 = fStack_3e4 * local_650;
              local_650 = local_3f8 * local_658 + local_3ec * fStack_654 + local_3e0 * local_650;
              local_670 = local_648;
              fStack_66c = fVar35 + local_3f0 * fStack_66c + fVar39;
              fStack_664 = local_3f4 * local_660 + fStack_664 * local_400 + fVar30;
              local_660 = fVar33 + local_3f0 * local_660 + fVar32;
              local_658 = local_3f4 * fStack_654 + local_658 * local_400 + fVar31;
              fStack_654 = fVar29 + local_3f0 * fStack_654 + fVar34;
              local_628 = local_668;
              FUN_140594980();
              local_5f8 = local_658;
              fStack_5f4 = fStack_654;
              local_5f0 = local_650;
              local_61c = local_650 * local_618 + local_630;
              local_620 = fStack_654 * local_618 + fStack_634;
              local_60c = local_638 + local_658 * local_618;
            }
          }
LAB_140144258:
          fVar29 = local_4b0;
          fVar32 = local_620;
          if (local_4b0 != 0.0) {
            FUN_14023a620(param_1[0x28],0x82);
            fVar34 = (float)FUN_140087a60();
            fVar33 = local_5f0;
            fVar30 = fStack_5f4;
            fVar39 = local_5f8;
            fVar31 = local_5f0 - fVar29 * DAT_141796e40 * fVar34;
            fVar35 = fStack_5f4 - fVar29 * DAT_141796e3c * fVar34;
            fVar34 = local_5f8 - fVar29 * DAT_141796e38 * fVar34;
            fVar29 = fVar34 * fVar34 + fVar35 * fVar35 + fVar31 * fVar31;
            if (fVar29 != 0.0) {
              fVar29 = sqrtf(fVar29);
              fVar29 = DAT_1415432f4 / fVar29;
              fVar34 = fVar34 * fVar29;
              fVar35 = fVar35 * fVar29;
              fVar31 = fVar31 * fVar29;
            }
            fVar29 = DAT_1415432f4;
            fVar36 = sqrtf(fVar39 * fVar39 + fVar30 * fVar30 + fVar33 * fVar33);
            fVar37 = sqrtf(fVar34 * fVar34 + fVar35 * fVar35 + fVar31 * fVar31);
            if (fVar37 * fVar36 == 0.0) {
              fVar37 = 0.0;
            }
            else {
              fVar36 = (fVar35 * fVar30 + fVar34 * fVar39 + fVar31 * fVar33) / (fVar37 * fVar36);
              if (fVar36 < fVar29) {
                fVar37 = DAT_14102c870;
                if (DAT_14102c874 < fVar36) {
                  fVar36 = acosf(fVar36);
                  fVar37 = fVar36 * DAT_14102c808;
                }
              }
              else {
                fVar37 = 0.0;
              }
            }
            local_490 = fVar37;
            if (DAT_1410458b8 < fVar37) {
              fVar32 = fVar35 * fVar39 - fVar34 * fVar30;
              fVar34 = fVar34 * fVar33 - fVar31 * fVar39;
              fVar30 = fVar31 * fVar30 - fVar35 * fVar33;
              fVar39 = fVar30 * fVar30 + fVar34 * fVar34 + fVar32 * fVar32;
              local_540 = fVar30;
              local_53c = fVar34;
              local_538 = fVar32;
              if (fVar39 != 0.0) {
                local_538 = sqrtf(fVar39);
                local_538 = fVar29 / local_538;
                local_540 = fVar30 * local_538;
                local_53c = fVar34 * local_538;
                local_538 = fVar32 * local_538;
              }
              local_400 = fVar29;
              fStack_3fc = 0.0;
              local_3f8 = 0.0;
              local_3f4 = 0.0;
              local_3f0 = fVar29;
              local_3ec = 0.0;
              local_3e8 = 0.0;
              fStack_3e4 = 0.0;
              local_3e0 = fVar29;
              FUN_140593bf0(&local_400,&local_540);
              local_648 = local_670 * local_400 + local_3f4 * fStack_66c + local_3e8 * local_668;
              fVar35 = local_670 * fStack_3fc;
              fVar29 = fStack_3e4 * local_668;
              local_668 = local_670 * local_3f8 + local_3ec * fStack_66c + local_3e0 * local_668;
              fVar39 = local_3e8 * fStack_65c;
              fVar33 = fStack_3fc * fStack_664;
              fVar30 = fStack_3e4 * fStack_65c;
              fStack_65c = local_3f8 * fStack_664 + local_3ec * local_660 + local_3e0 * fStack_65c;
              fVar31 = local_3e8 * local_650;
              fVar32 = fStack_3fc * local_658;
              fVar34 = fStack_3e4 * local_650;
              local_650 = local_3f8 * local_658 + local_3ec * fStack_654 + local_3e0 * local_650;
              local_670 = local_648;
              fStack_66c = fVar35 + local_3f0 * fStack_66c + fVar29;
              fStack_664 = local_400 * fStack_664 + local_3f4 * local_660 + fVar39;
              local_660 = fVar33 + local_3f0 * local_660 + fVar30;
              local_658 = local_400 * local_658 + local_3f4 * fStack_654 + fVar31;
              fStack_654 = fVar32 + local_3f0 * fStack_654 + fVar34;
              local_628 = local_668;
              FUN_140594980();
              local_5f8 = local_658;
              fStack_5f4 = fStack_654;
              local_5f0 = local_650;
              local_61c = local_650 * local_618 + local_630;
              fVar32 = fStack_654 * local_618 + fStack_634;
              local_60c = local_638 + local_658 * local_618;
              local_620 = fVar32;
            }
          }
          uVar18 = local_5c0;
          fVar29 = local_618;
          bVar5 = false;
          iVar9 = (int)local_5c0;
          fVar33 = local_61c;
          if ((iVar9 != 0) && (cVar6 == '\0')) {
            local_5d8 = CONCAT44(fStack_5f4,local_5f8);
            local_5d0 = local_5f0;
            fVar39 = *(float *)(param_1 + 0x135);
            local_624 = cVar6;
            local_648 = (float)FUN_1400fd260();
            local_648 = local_648 * fVar39;
            local_628 = 0.0;
            pfVar14 = &local_648;
            if (local_648 <= 0.0) {
              pfVar14 = &local_628;
            }
            local_608 = *pfVar14;
            if ((*(byte *)((longlong)param_1 + 0xb8c) & 0x20) != 0) {
              local_608 = fVar39;
            }
            uVar26 = 0;
            fVar33 = local_61c;
            fVar32 = local_620;
            if (iVar9 != 0) {
              uVar18 = uVar26;
              fVar39 = local_5d0;
              fVar30 = local_5d8._4_4_;
              fVar32 = (float)local_5d8;
              do {
                fVar33 = local_608;
                bVar4 = false;
                cVar7 = FUN_1402ca0c0(param_1[0x21] + uVar18 * 0x28,param_1);
                if (cVar7 != '\0') {
                  if ((local_614 == 0) && ((*(byte *)((longlong)param_1 + 0xb8c) & 1) != 0)) {
                    lVar19 = param_1[0x140];
                    fVar36 = local_630 - *(float *)(lVar19 + 0x94);
                    fVar31 = fStack_634 - *(float *)(lVar19 + 0x90);
                    fVar35 = local_638 - *(float *)(lVar19 + 0x8c);
                    fVar34 = fVar31 * fVar31 + fVar35 * fVar35 + fVar36 * fVar36;
                    local_560 = fVar35;
                    local_55c = fVar31;
                    local_558 = fVar36;
                    fVar29 = sqrtf(fVar34);
                    if (fVar34 != 0.0) {
                      local_558 = sqrtf(fVar34);
                      local_558 = DAT_1415432f4 / local_558;
                      local_560 = fVar35 * local_558;
                      local_55c = fVar31 * local_558;
                      local_558 = local_558 * fVar36;
                    }
                    local_690 = (float *)((ulonglong)local_690 & 0xffffffff00000000);
                    local_698 = (float *)CONCAT44(local_698._4_4_,fVar33);
                    local_6a0 = (float *)CONCAT44(local_6a0._4_4_,fVar33);
                    local_6a8 = (float *)CONCAT44(local_6a8._4_4_,fVar29);
                    iVar8 = FUN_1401f1900(*(undefined8 *)(uVar26 + 0x10 + param_1[0x21]),&local_560,
                                          (float *)(lVar19 + 0x8c),&local_560);
                    fVar29 = local_618;
                    cVar6 = local_624;
                    if (iVar8 != 0) {
                      FUN_140134a50(param_1);
                      FUN_1400c7a80(local_3d8);
                      goto LAB_1401453fb;
                    }
                  }
                  local_690 = (float *)CONCAT44(local_690._4_4_,local_5ac);
                  local_698 = (float *)CONCAT44(local_698._4_4_,local_600);
                  local_6a0 = (float *)CONCAT44(local_6a0._4_4_,fVar33);
                  local_6a8 = (float *)CONCAT44(local_6a8._4_4_,fVar29);
                  iVar8 = FUN_1401f1900(*(undefined8 *)(uVar26 + 0x10 + param_1[0x21]),&local_4d0,
                                        &local_638,&local_5f8);
                  if (iVar8 != 0) {
                    if (iVar8 == 1) {
                      bVar4 = true;
                    }
                    else if (iVar8 == 2) {
                      local_678 = '\x01';
                    }
                    else {
                      if (iVar8 != 3) {
                        if (iVar8 != 4) {
                          (**(code **)(*param_1 + 8))(param_1,&local_400);
                          FUN_14059f080(DAT_141a9c730,
                                        "<error>[INTERNAL ERROR - \'%s\']Invalid collision type.\n")
                          ;
                          if (0xf < CONCAT44(fStack_3e4,local_3e8)) {
                            operator_delete((void *)CONCAT44(fStack_3fc,local_400));
                          }
                          goto LAB_140144eda;
                        }
                        FUN_140134a50(param_1);
                        FUN_1400c7a80(local_3d8);
                        goto LAB_1401453fb;
                      }
                      bVar5 = true;
                    }
                    puVar1 = (uint *)((longlong)local_488 + (uVar18 >> 5) * 4);
                    *puVar1 = *puVar1 | 1 << ((byte)uVar18 & 0x1f);
                    lVar19 = *(longlong *)(param_1[0x21] + 0x10 + uVar26);
                    if ((*(int *)(lVar19 + 0x24c) != 0) && (*(int *)(lVar19 + 0x250) == 2)) {
                      FUN_140134a50(param_1);
                      FUN_1400c7a80(local_3d8);
                      goto LAB_1401453fb;
                    }
                    if (bVar4) {
                      cVar6 = '\x01';
                      local_624 = '\x01';
                      fVar36 = sqrtf(fVar32 * fVar32 + fVar30 * fVar30 + fVar39 * fVar39);
                      fVar35 = local_5f0;
                      fVar31 = fStack_5f4;
                      fVar34 = local_5f8;
                      fVar37 = sqrtf(local_5f8 * local_5f8 + fStack_5f4 * fStack_5f4 +
                                     local_5f0 * local_5f0);
                      fVar33 = DAT_1415432f4;
                      if (fVar37 * fVar36 == 0.0) {
                        fVar37 = 0.0;
                      }
                      else {
                        fVar36 = (fVar30 * fVar31 + fVar34 * fVar32 + fVar39 * fVar35) /
                                 (fVar37 * fVar36);
                        if (fVar36 < DAT_1415432f4) {
                          fVar37 = DAT_14102c870;
                          if (DAT_14102c874 < fVar36) {
                            fVar36 = acosf(fVar36);
                            fVar37 = fVar36 * DAT_14102c808;
                          }
                        }
                        else {
                          fVar37 = 0.0;
                        }
                      }
                      local_648 = fVar37;
                      if (DAT_1410458b8 < fVar37) {
                        fVar36 = fVar31 * fVar32 - fVar30 * fVar34;
                        fVar32 = fVar39 * fVar34 - fVar35 * fVar32;
                        fVar39 = fVar30 * fVar35 - fVar39 * fVar31;
                        fVar29 = fVar39 * fVar39 + fVar32 * fVar32 + fVar36 * fVar36;
                        local_530 = fVar39;
                        local_52c = fVar32;
                        local_528 = fVar36;
                        if (fVar29 != 0.0) {
                          local_528 = sqrtf(fVar29);
                          local_528 = fVar33 / local_528;
                          local_530 = fVar39 * local_528;
                          local_52c = fVar32 * local_528;
                          local_528 = fVar36 * local_528;
                        }
                        local_400 = fVar33;
                        fStack_3fc = 0.0;
                        local_3f8 = 0.0;
                        local_3f4 = 0.0;
                        local_3f0 = fVar33;
                        local_3ec = 0.0;
                        local_3e8 = 0.0;
                        fStack_3e4 = 0.0;
                        local_3e0 = fVar33;
                        FUN_140593bf0(&local_400,&local_530);
                        local_628 = local_3f4 * fStack_66c + local_670 * local_400 +
                                    local_3e8 * local_668;
                        fVar35 = local_670 * fStack_3fc;
                        fVar39 = fStack_3e4 * local_668;
                        local_668 = local_670 * local_3f8 + local_3ec * fStack_66c +
                                    local_3e0 * local_668;
                        fVar30 = local_3e8 * fStack_65c;
                        fVar33 = fStack_3fc * fStack_664;
                        fVar32 = fStack_3e4 * fStack_65c;
                        fStack_65c = local_3f8 * fStack_664 + local_3ec * local_660 +
                                     local_3e0 * fStack_65c;
                        fVar31 = local_3e8 * local_650;
                        fVar29 = fStack_3fc * local_658;
                        fVar34 = fStack_3e4 * local_650;
                        local_650 = local_3f8 * local_658 + local_3ec * fStack_654 +
                                    local_3e0 * local_650;
                        local_670 = local_628;
                        fStack_66c = fVar35 + local_3f0 * fStack_66c + fVar39;
                        fStack_664 = local_3f4 * local_660 + fStack_664 * local_400 + fVar30;
                        local_660 = fVar33 + local_3f0 * local_660 + fVar32;
                        local_658 = local_3f4 * fStack_654 + local_658 * local_400 + fVar31;
                        fStack_654 = fVar29 + local_3f0 * fStack_654 + fVar34;
                        local_5e8 = local_668;
                        FUN_140594980();
                        fVar39 = local_5d0;
                        fVar30 = local_5d8._4_4_;
                        fVar32 = (float)local_5d8;
                        fVar29 = local_618;
                      }
                    }
                  }
                }
LAB_140144eda:
                if ((local_678 != '\0') &&
                   (local_640[0] = (float)(local_614 + 1), local_584 <= (int)local_640[0])) {
                  local_640[0] = -NAN;
                }
                uVar28 = (int)uVar18 + 1;
                uVar18 = (ulonglong)uVar28;
                uVar26 = uVar26 + 0x28;
              } while (uVar28 < (uint)local_5c0);
              fVar33 = local_61c;
              fVar32 = local_620;
              if (cVar6 != '\0') {
                local_60c = local_638 + local_658 * fVar29;
                fVar33 = local_630 + local_650 * fVar29;
                fVar32 = fStack_634 + fStack_654 * fVar29;
              }
              uVar18 = local_5c0;
              iVar8 = local_614;
              if ((bVar5) &&
                 (*(float *)(param_1 + 0x133) <= 0.0 && *(float *)(param_1 + 0x133) != 0.0)) {
                *(float *)(param_1 + 0x133) = local_604;
                fVar29 = local_604 * *(float *)((longlong)param_1 + 0x99c);
                *(float *)((longlong)param_1 + 0x99c) = fVar29;
                *(float *)(param_1 + 0x134) = fVar29;
              }
            }
          }
          local_638 = local_60c;
          local_614 = iVar8 + 1;
          local_418 = local_418 + 1;
          uVar26 = local_578;
          fVar39 = local_618;
          fVar29 = DAT_1415432f4;
          fVar30 = local_5e4;
          fStack_634 = fVar32;
          local_630 = fVar33;
          fVar33 = local_5b4;
        } while (local_614 < local_584);
      }
      uVar16 = 0;
      local_638 = (float)local_570;
      fStack_634 = local_570._4_4_;
      local_630 = local_568;
      local_670 = local_440;
      fStack_66c = fStack_43c;
      local_668 = local_438;
      fStack_664 = fStack_434;
      local_660 = local_430;
      fStack_65c = fStack_42c;
      local_658 = local_428;
      fStack_654 = fStack_424;
      local_650 = local_420;
      bVar5 = false;
      iVar9 = 0;
      iVar8 = 0;
      uVar22 = uVar16;
      uVar24 = uVar16;
      uVar27 = uVar16;
      bVar4 = false;
      if ((int)uVar18 != 0) {
        do {
          bVar5 = bVar4;
          cVar6 = FUN_1402ca0c0(uVar27 + param_1[0x21],param_1,0);
          if ((cVar6 == '\0') ||
             (iVar8 = FUN_14059b650(*(undefined8 *)
                                     (*(longlong *)(uVar24 + 0x10 + param_1[0x21]) + 0xf8)),
             iVar8 != 6)) goto LAB_1401451d7;
          lVar19 = *(longlong *)(param_1[0x21] + 0x10 + uVar24);
          if (*(int *)(lVar19 + 0x24c) == 0) {
LAB_1401451d5:
            uVar16 = (ulonglong)((int)uVar16 + 1);
          }
          else if (*(int *)(lVar19 + 0x250) == 1) {
            bVar5 = true;
            if (((longlong)uVar22 < 0) && (uVar22 != 0)) {
              lVar19 = -((-uVar22 - 1 >> 5) * 4 + 4);
            }
            else {
              lVar19 = (uVar22 >> 5) * 4;
            }
            if ((*(uint *)((longlong)local_488 + lVar19) & 1 << ((byte)uVar22 & 0x1f)) != 0)
            goto LAB_1401451d5;
          }
          else if ((*(int *)(lVar19 + 0x24c) == 0) || (*(int *)(lVar19 + 0x250) == 0))
          goto LAB_1401451d5;
LAB_1401451d7:
          iVar9 = (int)uVar16;
          uVar26 = uVar26 - 1;
          uVar22 = uVar22 + 1;
          uVar24 = uVar24 + 0x28;
          uVar27 = uVar27 + 0x28;
          bVar4 = bVar5;
        } while (uVar26 != 0);
        iVar8 = (int)local_5c0;
      }
      if (((local_644 != '\0') && ((*(byte *)((longlong)param_1 + 0x1ee) & 1) == 0)) ||
         ((bVar5 && (iVar9 == 0)))) {
        FUN_140134a50(param_1);
        if (local_340 != '\0') {
          if (local_358 != (undefined8 *)0x0) {
            (**(code **)*local_358)(local_358,1);
          }
          local_358 = (undefined8 *)0x0;
          if (local_348 != (undefined8 *)0x0) {
            (**(code **)*local_348)(local_348,1);
          }
          local_348 = (undefined8 *)0x0;
        }
        FUN_140050c50(local_380);
        if (0xf < local_3a0) {
          operator_delete((void *)CONCAT71(uStack_3b7,local_3b8));
        }
        local_3a0 = 0xf;
        local_3a8 = 0;
        local_3b8 = 0;
      }
      else {
        FUN_1400f5130(param_1,local_640[0]);
        fVar29 = (float)FUN_1402e3050(param_1[0x2a] + 0x300,param_1 + 0x38);
        if (0.0 < fVar29 * *(float *)((longlong)param_1 + 0x99c) * DAT_14102cfc8) {
          if (iVar8 != 0) {
            uVar18 = 0;
            do {
              if (((longlong)uVar18 < 0) && (uVar18 != 0)) {
                lVar19 = -((-uVar18 - 1 >> 5) * 4 + 4);
              }
              else {
                lVar19 = (uVar18 >> 5) * 4;
              }
              if ((*(uint *)((longlong)local_488 + lVar19) & 1 << ((byte)uVar18 & 0x1f)) != 0) {
                FUN_1402c9b20(param_1,
                              "1Late noise may have moved this branch off of or into a collision mesh."
                             );
              }
              uVar18 = uVar18 + 1;
              local_578 = local_578 - 1;
            } while (local_578 != 0);
          }
          FUN_1400fb480(param_1);
        }
        FUN_1400fbf60(param_1);
        if (local_340 != '\0') {
          if (local_358 != (undefined8 *)0x0) {
            (**(code **)*local_358)(local_358,1);
          }
          local_358 = (undefined8 *)0x0;
          if (local_348 != (undefined8 *)0x0) {
            (**(code **)*local_348)(local_348,1);
          }
          local_348 = (undefined8 *)0x0;
        }
        FUN_140050c50(local_380);
        if (0xf < local_3a0) {
          operator_delete((void *)CONCAT71(uStack_3b7,local_3b8));
        }
        local_3a0 = 0xf;
        local_3a8 = 0;
        local_3b8 = 0;
      }
LAB_1401453fb:
      if (local_488 == (void *)0x0) {
        return;
      }
      operator_delete(local_488);
      return;
    }
    if ((*(byte *)((longlong)param_1 + 0x1ee) & 1) == 0) goto LAB_140141eb4;
    pcVar17 = "1Branch extension has a length of 0.0.";
  }
  else {
    pcVar17 = "1Parent too short to support branch (branch removed).";
  }
  FUN_1402c9b20(param_1,pcVar17);
LAB_140141eb4:
  *(byte *)((longlong)param_1 + 0x1ec) = *(byte *)((longlong)param_1 + 0x1ec) & 0xfd;
  FUN_1401349c0(param_1);
  return;
}


